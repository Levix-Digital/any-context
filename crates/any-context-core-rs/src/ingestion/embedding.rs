//! The embedding model and the reindex rule (ADR-0138, ADR-0154).
//!
//! One embedding model, chosen for the whole application and independent of the chat model,
//! embeds every chunk and every query: the `embedding_model` setting, else the model already used
//! by the existing index (adopted once on upgrade), else the default local model of the hardware
//! profile. The model is never swapped silently: when it cannot run, callers get an
//! [`EmbeddingUnavailable`] naming the model, the cause and the fix (IMP-028).
//!
//! A workspace's vectors are only comparable with query vectors of the same vector space: same
//! provider, model, dimension and prefixes ([`EmbeddingTarget`]). The sync records the target that
//! built a workspace's vectors and reindexes the whole workspace when the target changes; queries
//! use the recorded target until then.

use std::fmt;
use std::sync::Arc;

use crate::hardware::{HardwareProfiler, ProfileTier};
use crate::ingestion::embedding_catalog::{
    apply_prefix, find_embedding_model, spec_for_target, EmbedRole, EmbeddingModelSpec, EmbeddingRuntime,
};
use crate::ingestion::local_embedding::LocalEmbeddingProvider;
use crate::ingestion::OnnxModelManager;
use crate::storage::sqlite::NativeConfigDb;

use actx_lm::providers::gemini::GeminiProvider;
use actx_lm::providers::mock::MockLmProvider;
use actx_lm::providers::openai::OpenAiCompatibleProvider;
use actx_lm::traits::LmProvider;

/// Setting with the catalog id of the embedding model chosen by the user (or adopted from the
/// existing index on upgrade). Absent: the profile's default.
pub const EMBEDDING_MODEL_SETTING: &str = "embedding_model";
/// Marks the one-time adoption of the existing index's model as done.
pub const EMBEDDING_ADOPTION_SETTING: &str = "embedding_model_adopted";

/// Setting with the target of the `Global` workspace's vectors. Empty or absent while they are
/// the placeholders written by the startup bootstrap (BUG-019).
pub const GLOBAL_KNOWLEDGE_TARGET_SETTING: &str = "global_knowledge_embedding_target";

/// Gemini embedding model used before v0.45.1, shut down by Google on 2026-01-14.
pub const LEGACY_GEMINI_EMBEDDING_MODEL: &str = "text-embedding-004";

const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// The vector space of a workspace's vectors: provider, model, dimension and prefix revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingTarget {
    /// `local`, `openai`, `gemini`, `ollama`, or `mock` in tests.
    pub provider: String,
    /// Model id.
    pub model: String,
    pub dimension: usize,
    /// Revision of the model's prefixes; 0 = no prefixes.
    pub prefix_rev: u32,
}

impl EmbeddingTarget {
    pub fn new(provider: &str, model: &str, dimension: usize, prefix_rev: u32) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
            dimension,
            prefix_rev,
        }
    }

    /// The deterministic mock target: every text gets the same vector, so its vectors carry no
    /// meaning and are never searched (BUG-019).
    pub fn is_mock(&self) -> bool {
        self.provider == "mock"
    }

    /// Stable text form stored in SQLite and on each chunk, e.g.
    /// `local/multilingual-e5-small@384+p1` or `openai/text-embedding-3-small@1536`.
    pub fn key(&self) -> String {
        let mut key = format!("{}/{}@{}", self.provider, self.model, self.dimension);
        if self.prefix_rev > 0 {
            key.push_str(&format!("+p{}", self.prefix_rev));
        }
        key
    }

    /// Parses [`EmbeddingTarget::key`], and the `provider/model` keys recorded before ADR-0154,
    /// which get the dimension the old resolver produced and no prefixes.
    pub fn parse(key: &str) -> Option<Self> {
        let (provider, rest) = key.trim().split_once('/')?;
        if provider.is_empty() || rest.is_empty() {
            return None;
        }
        let Some((model, space)) = rest.rsplit_once('@') else {
            return Some(Self::new(provider, rest, legacy_dimension(provider, rest), 0));
        };
        let (dimension, prefix_rev) = match space.split_once("+p") {
            Some((d, p)) => (d.parse().ok()?, p.parse().ok()?),
            None => (space.parse().ok()?, 0),
        };
        if model.is_empty() {
            return None;
        }
        Some(Self::new(provider, model, dimension, prefix_rev))
    }
}

/// Vector size of a target recorded before ADR-0154 (the LanceDB table width for API models).
fn legacy_dimension(provider: &str, model: &str) -> usize {
    match (provider, model) {
        ("ollama", _) => 768,
        ("gemini", LEGACY_GEMINI_EMBEDDING_MODEL) => 768,
        _ => crate::storage::lancedb::DEFAULT_VECTOR_DIM,
    }
}

impl fmt::Display for EmbeddingTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.key())
    }
}

/// The embedding model cannot run: unknown, refused, missing key, missing or unverified files,
/// ONNX Runtime not loadable. Shown to the user as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingUnavailable {
    pub model: String,
    pub cause: String,
    pub fix: String,
}

impl EmbeddingUnavailable {
    pub fn new(model: impl Into<String>, cause: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            cause: cause.into(),
            fix: fix.into(),
        }
    }
}

impl fmt::Display for EmbeddingUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Embedding model {} is unavailable: {}. {}", self.model, self.cause, self.fix)
    }
}

impl std::error::Error for EmbeddingUnavailable {}

/// Where the active embedding model comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingModelSource {
    /// Chosen with `/embedding` (or adopted from the existing index on upgrade).
    Setting,
    /// Default of the hardware profile.
    Profile(ProfileTier),
    /// Explicit for this run (tests and the benchmark).
    Override,
}

impl EmbeddingModelSource {
    pub fn describe(&self) -> String {
        match self {
            Self::Setting => "chosen setting".to_string(),
            Self::Profile(tier) => format!("default of the {} profile", tier.display_name()),
            Self::Override => "explicit for this run".to_string(),
        }
    }
}

/// What a sync pass must do with the vectors already stored for a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReindexDecision {
    /// The recorded target is the current one.
    Keep,
    /// Nothing to repair: record the current target.
    Adopt,
    /// The stored vectors come from another target: purge and re-embed the whole workspace.
    Reindex { from: String, to: EmbeddingTarget },
}

impl ReindexDecision {
    /// Message shown to the user when the decision is a reindex.
    pub fn message(&self) -> Option<String> {
        match self {
            Self::Reindex { from, to } => Some(format!(
                "Embedding model changed ({} → {}): the whole workspace is reindexed.",
                from,
                to.key()
            )),
            _ => None,
        }
    }
}

/// Whether `spec` may be used on this installation. Every catalog model is allowed in this
/// release; the organization policy (IMP-056) plugs in here. A refused model stops the sync with
/// the reason; it is never swapped for another (FR-019).
pub fn is_model_allowed(spec: &EmbeddingModelSpec) -> Result<(), String> {
    let _ = spec;
    Ok(())
}

/// The default embedding model of a hardware profile (FR-003).
pub fn profile_default_model(tier: ProfileTier) -> &'static EmbeddingModelSpec {
    find_embedding_model(tier.default_embedding_model()).expect("profile default must be in the embedding catalog")
}

/// The hardware profile in effect: the `active_profile` setting, else the recommended one.
pub fn active_profile(db: &NativeConfigDb) -> ProfileTier {
    db.get_setting("active_profile")
        .ok()
        .flatten()
        .and_then(|p| ProfileTier::parse_str(&p))
        .unwrap_or_else(|| HardwareProfiler::auto_select_recommended(&HardwareProfiler::probe_system(None)))
}

/// The active embedding model and where it comes from, without building it.
///
/// Order: `override_id` (tests, benchmark), the `embedding_model` setting, the model of the
/// existing index (adopted once, so an upgrade does not reindex a working cloud index), the
/// profile's default. Never the mock, unless the override asks for it.
pub fn active_embedding_model(
    db: &NativeConfigDb,
    override_id: Option<&str>,
) -> Result<(&'static EmbeddingModelSpec, EmbeddingModelSource), EmbeddingUnavailable> {
    if let Some(id) = override_id.map(str::trim).filter(|id| !id.is_empty()) {
        let spec = find_embedding_model(id).ok_or_else(|| unknown_model(id))?;
        return Ok((spec, EmbeddingModelSource::Override));
    }
    adopt_existing_index_model(db);
    if let Some(id) = db.get_setting(EMBEDDING_MODEL_SETTING).ok().flatten().filter(|s| !s.trim().is_empty()) {
        let spec = find_embedding_model(&id)
            .filter(|s| !s.is_mock())
            .ok_or_else(|| unknown_model(&id))?;
        return Ok((spec, EmbeddingModelSource::Setting));
    }
    let tier = active_profile(db);
    Ok((profile_default_model(tier), EmbeddingModelSource::Profile(tier)))
}

fn unknown_model(id: &str) -> EmbeddingUnavailable {
    let ids: Vec<&str> = crate::ingestion::embedding_catalog::selectable_models().map(|m| m.id).collect();
    EmbeddingUnavailable::new(id, "it is not in the embedding catalog", format!("Choose one of {} with /embedding.", ids.join(", ")))
}

/// One-time upgrade step (ADR-0154): when no embedding model was ever chosen, the model of the
/// existing index becomes the setting, so a working cloud index is not reindexed by surprise.
/// The most used real (non-mock) target wins; mock and shut-down models are ignored.
pub fn adopt_existing_index_model(db: &NativeConfigDb) {
    if db.get_setting(EMBEDDING_ADOPTION_SETTING).ok().flatten().is_some() {
        return;
    }
    if db.get_setting(EMBEDDING_MODEL_SETTING).ok().flatten().filter(|s| !s.trim().is_empty()).is_none() {
        if let Some(id) = most_used_recorded_model(db) {
            let _ = db.set_setting(EMBEDDING_MODEL_SETTING, id);
        }
    }
    let _ = db.set_setting(EMBEDDING_ADOPTION_SETTING, &chrono::Utc::now().to_rfc3339());
}

fn most_used_recorded_model(db: &NativeConfigDb) -> Option<&'static str> {
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for ws in db.list_workspace_names().unwrap_or_default() {
        let Some(key) = db.get_workspace_embedding_target(&ws).ok().flatten().flatten() else {
            continue;
        };
        let Some(spec) = EmbeddingTarget::parse(&key).as_ref().and_then(spec_for_target) else {
            continue;
        };
        if spec.is_mock() {
            continue;
        }
        match counts.iter_mut().find(|(id, _)| *id == spec.id) {
            Some((_, n)) => *n += 1,
            None => counts.push((spec.id, 1)),
        }
    }
    // Stable: on a tie, the first workspace listed wins.
    counts.iter().fold(None, |best: Option<(&'static str, usize)>, &(id, n)| match best {
        Some((_, m)) if m >= n => best,
        _ => Some((id, n)),
    }).map(|(id, _)| id)
}

/// Builds the provider of `spec` without any fallback.
pub fn build_embedding_provider(db: &NativeConfigDb, spec: &EmbeddingModelSpec) -> Result<Arc<dyn LmProvider>, EmbeddingUnavailable> {
    build_provider_parts(db, spec).map(|(provider, _)| provider)
}

/// The provider, plus the ONNX embedder behind it for a local model (to read its counters).
fn build_provider_parts(
    db: &NativeConfigDb,
    spec: &EmbeddingModelSpec,
) -> Result<(Arc<dyn LmProvider>, Option<actx_onnx::OnnxEmbedder>), EmbeddingUnavailable> {
    let cloud = |p: Arc<dyn LmProvider>| -> Result<(Arc<dyn LmProvider>, Option<actx_onnx::OnnxEmbedder>), EmbeddingUnavailable> { Ok((p, None)) };
    is_model_allowed(spec).map_err(|reason| EmbeddingUnavailable::new(spec.id, reason, "Choose an allowed model with /embedding."))?;
    match spec.runtime {
        EmbeddingRuntime::Mock => cloud(Arc::new(MockLmProvider::new())),
        EmbeddingRuntime::Cloud("ollama") => cloud(Arc::new(OpenAiCompatibleProvider::new("ollama", OLLAMA_BASE_URL.to_string(), None))),
        EmbeddingRuntime::Cloud("gemini") => {
            let key = db.get_api_key("gemini").ok().flatten().ok_or_else(|| missing_key(spec, "gemini"))?;
            cloud(Arc::new(GeminiProvider::new(key)))
        }
        EmbeddingRuntime::Cloud(provider) => {
            let key = db.get_api_key(provider).ok().flatten().ok_or_else(|| missing_key(spec, provider))?;
            cloud(Arc::new(OpenAiCompatibleProvider::new(provider, OPENAI_BASE_URL.to_string(), Some(key))))
        }
        EmbeddingRuntime::Local => {
            let files = OnnxModelManager::find_spec(spec.id)
                .ok_or_else(|| EmbeddingUnavailable::new(spec.id, "its files are not in the model catalog", "Update AnyContext."))?;
            if !actx_onnx::runtime::runtime_library_present() {
                return Err(EmbeddingUnavailable::new(
                    spec.id,
                    "the ONNX Runtime library is not available",
                    "Reinstall AnyContext so the bundled ONNX Runtime is restored.",
                ));
            }
            OnnxModelManager::verify_installed(files).map_err(|cause| EmbeddingUnavailable::new(spec.id, cause, install_fix(files)))?;
            let provider = LocalEmbeddingProvider::for_spec(spec);
            provider
                .embedder()
                .ensure_loaded()
                .map_err(|cause| EmbeddingUnavailable::new(spec.id, cause, install_fix(files)))?;
            let embedder = provider.embedder().clone();
            Ok((Arc::new(provider), Some(embedder)))
        }
    }
}

fn missing_key(spec: &EmbeddingModelSpec, provider: &str) -> EmbeddingUnavailable {
    EmbeddingUnavailable::new(
        spec.id,
        format!("no {provider} API key is configured"),
        format!("Add the key with /keys {provider} <key>, or choose a local model with /embedding."),
    )
}

/// How to install a local model by hand: run the download, or copy the files from the source.
fn install_fix(files: &crate::ingestion::OnnxModelSpec) -> String {
    let mirror = OnnxModelManager::models_mirror();
    let sources: Vec<String> = files.files.iter().map(|f| OnnxModelManager::source_url(f, mirror.as_deref())).collect();
    format!(
        "Run `actx download {}` with access to {}, or copy the verified files into {}.",
        files.id,
        sources.join(" and "),
        crate::storage::sqlite::get_default_models_dir().display()
    )
}

/// The embedding model of a sync, built and ready.
#[derive(Clone)]
pub struct ResolvedEmbedding {
    pub spec: &'static EmbeddingModelSpec,
    pub target: EmbeddingTarget,
    pub source: EmbeddingModelSource,
    pub provider: Arc<dyn LmProvider>,
    /// The ONNX embedder of a local model, for its truncation counter.
    pub local: Option<actx_onnx::OnnxEmbedder>,
}

impl ResolvedEmbedding {
    /// A resolved embedding around an explicit provider (tests).
    pub fn with_provider(spec: &'static EmbeddingModelSpec, provider: Arc<dyn LmProvider>) -> Self {
        Self {
            spec,
            target: spec.target(),
            source: EmbeddingModelSource::Override,
            provider,
            local: None,
        }
    }

    /// Inputs cut at the model's limit so far (local models).
    pub fn truncated_inputs(&self) -> usize {
        self.local.as_ref().map(|e| e.truncated_inputs()).unwrap_or(0)
    }

    /// Text sent to the model for a chunk: the document prefix plus the text (IMP-029).
    pub fn document_text(&self, text: &str) -> String {
        apply_prefix(self.spec, &self.target, EmbedRole::Document, text)
    }
}

impl fmt::Debug for ResolvedEmbedding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedEmbedding").field("target", &self.target.key()).field("source", &self.source).finish()
    }
}

/// Resolves and builds the embedding model of a sync. A missing local model is downloaded first
/// through the verified download (from the configured mirror when there is one); every failure is
/// an [`EmbeddingUnavailable`], never another model.
pub async fn prepare_embedding(db: &NativeConfigDb, override_id: Option<&str>) -> Result<ResolvedEmbedding, EmbeddingUnavailable> {
    let (spec, source) = active_embedding_model(db, override_id)?;
    if spec.is_local() {
        if let Some(files) = OnnxModelManager::find_spec(spec.id) {
            if !OnnxModelManager::is_installed(files) {
                OnnxModelManager::download_model(files)
                    .await
                    .map_err(|cause| EmbeddingUnavailable::new(spec.id, format!("download failed: {cause}"), install_fix(files)))?;
            }
        }
    }
    let (provider, local) = build_provider_parts(db, spec)?;
    Ok(ResolvedEmbedding {
        spec,
        target: spec.target(),
        source,
        provider,
        local,
    })
}

/// The query side: provider, target and prefix for question vectors of a workspace.
#[derive(Clone)]
pub struct QueryEmbedding {
    /// `None`: dense search is skipped for this workspace (mock index, or the model cannot run).
    pub provider: Option<Arc<dyn LmProvider>>,
    pub target: EmbeddingTarget,
    pub spec: Option<&'static EmbeddingModelSpec>,
    /// Why there is no provider, when it is not the mock.
    pub unavailable: Option<String>,
}

impl QueryEmbedding {
    /// Text sent to the model for a question: the query prefix of the target's space plus the
    /// question (IMP-029).
    pub fn query_text(&self, question: &str) -> String {
        match self.spec {
            Some(spec) => apply_prefix(spec, &self.target, EmbedRole::Query, question),
            None => question.to_string(),
        }
    }

    /// The prefix put in front of questions, or `""`.
    pub fn query_prefix(&self) -> String {
        let probe = self.query_text("");
        probe
    }
}

impl fmt::Debug for QueryEmbedding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryEmbedding")
            .field("target", &self.target.key())
            .field("provider", &self.provider.is_some())
            .field("unavailable", &self.unavailable)
            .finish()
    }
}

/// Provider and target for the question vectors of `workspace`.
///
/// The target recorded by the last completed sync answers until the next sync reindexes the
/// workspace, so a model change does not break dense search in between. When that model cannot
/// run, dense search is skipped for the question (lexical only, the cause recorded); a question
/// is never embedded by another model. Never downloads: the sync does.
pub fn resolve_query_embedding(db: &NativeConfigDb, workspace: &str) -> QueryEmbedding {
    let recorded = db
        .get_workspace_embedding_target(workspace)
        .ok()
        .flatten()
        .flatten()
        .and_then(|key| EmbeddingTarget::parse(&key));
    let (target, spec) = match recorded {
        Some(target) => {
            let spec = spec_for_target(&target);
            (target, spec)
        }
        None => match active_embedding_model(db, None) {
            Ok((spec, _)) => (spec.target(), Some(spec)),
            Err(e) => {
                return QueryEmbedding {
                    provider: None,
                    target: EmbeddingTarget::new("none", &e.model, 0, 0),
                    spec: None,
                    unavailable: Some(e.to_string()),
                }
            }
        },
    };
    if target.is_mock() {
        return QueryEmbedding { provider: None, target, spec, unavailable: None };
    }
    let built = match spec {
        Some(spec) if spec.dimension == target.dimension => build_embedding_provider(db, spec).map_err(|e| e.to_string()),
        Some(spec) => Err(format!("{} now has {} dimensions, the index {}", spec.id, spec.dimension, target.dimension)),
        None => Err(format!("{} can no longer be used; sync the workspace to reindex it", target.key())),
    };
    match built {
        Ok(provider) => QueryEmbedding { provider: Some(provider), target, spec, unavailable: None },
        Err(cause) => QueryEmbedding { provider: None, target, spec, unavailable: Some(cause) },
    }
}

/// Whether dense search may include the `Global` workspace for queries on `query_target`: only
/// when its vectors were embedded with that target, never on the mock target (BUG-019).
pub fn global_dense_enabled(db: &NativeConfigDb, query_target: &EmbeddingTarget) -> bool {
    !query_target.is_mock()
        && db
            .get_setting(GLOBAL_KNOWLEDGE_TARGET_SETTING)
            .ok()
            .flatten()
            .and_then(|k| EmbeddingTarget::parse(&k))
            .as_ref()
            == Some(query_target)
}

/// Notice for the command layer when the next sync of a workspace will reindex it, e.g. right
/// after `/embedding` changed the model. Uses the SQLite stat cache as the "indexed" signal.
pub fn pending_reindex_notice(db: &NativeConfigDb, workspace: &str) -> Option<String> {
    let recorded = db.get_workspace_embedding_target(workspace).ok().flatten()?;
    let (spec, _) = active_embedding_model(db, None).ok()?;
    let has_indexed_data = !db.get_workspace_files_stat_cache(workspace).unwrap_or_default().is_empty();
    match decide_reindex(recorded.as_deref(), &spec.target(), has_indexed_data) {
        ReindexDecision::Reindex { from, to } => Some(format!(
            "Embedding model changes ({} → {}): the next sync reindexes the whole workspace.",
            from,
            to.key()
        )),
        _ => None,
    }
}

/// Workspaces whose vectors a switch to `target` would reindex, with their recorded target.
pub fn workspaces_reindexed_by(db: &NativeConfigDb, target: &EmbeddingTarget) -> Vec<(String, String)> {
    db.list_workspace_names()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|ws| {
            let key = db.get_workspace_embedding_target(&ws).ok().flatten().flatten()?;
            match EmbeddingTarget::parse(&key) {
                Some(t) if &t == target => None,
                _ => Some((ws, key)),
            }
        })
        .collect()
}

/// Decides whether the stored vectors of a workspace can be kept.
///
/// `recorded` is the stored target key (`None` for a workspace indexed before the record
/// existed) and `has_indexed_data` whether the workspace has any stored chunk. Keys are compared
/// as vector spaces, so a key recorded before ADR-0154 for the same space is kept.
pub fn decide_reindex(recorded: Option<&str>, current: &EmbeddingTarget, has_indexed_data: bool) -> ReindexDecision {
    match recorded {
        Some(key) if EmbeddingTarget::parse(key).as_ref() == Some(current) => ReindexDecision::Keep,
        Some(key) => ReindexDecision::Reindex {
            from: key.to_string(),
            to: current.clone(),
        },
        None if !has_indexed_data => ReindexDecision::Adopt,
        // Indexed before targets were recorded: only the old OpenAI default can be kept as is.
        None if current == &EmbeddingTarget::new("openai", "text-embedding-3-small", 1536, 0) => ReindexDecision::Adopt,
        None => ReindexDecision::Reindex {
            from: "an unrecorded model".to_string(),
            to: current.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(key: &str) -> EmbeddingTarget {
        EmbeddingTarget::parse(key).expect("valid key")
    }

    #[test]
    fn key_round_trips_and_legacy_keys_parse() {
        let e5 = EmbeddingTarget::new("local", "multilingual-e5-small", 384, 1);
        assert_eq!(e5.key(), "local/multilingual-e5-small@384+p1");
        assert_eq!(EmbeddingTarget::parse(&e5.key()), Some(e5));
        let openai = EmbeddingTarget::new("openai", "text-embedding-3-small", 1536, 0);
        assert_eq!(openai.key(), "openai/text-embedding-3-small@1536");
        assert_eq!(target("openai/text-embedding-3-small"), openai, "legacy key, same space");
        assert_eq!(target("ollama/nomic-embed-text"), EmbeddingTarget::new("ollama", "nomic-embed-text", 768, 0));
        assert_eq!(EmbeddingTarget::parse("no-separator"), None);
        assert_eq!(EmbeddingTarget::parse("/model"), None);
        assert_eq!(EmbeddingTarget::parse("local/@384"), None);
        assert_eq!(EmbeddingTarget::parse("local/m@x"), None);
    }

    #[test]
    fn spaces_differ_by_dimension_and_prefixes() {
        assert_ne!(target("local/m@384+p1"), target("local/m@384"));
        assert_ne!(target("local/m@384"), target("local/m@768"));
        assert_ne!(target("gemini/gemini-embedding-2"), target("gemini/gemini-embedding-2@1536+p1"));
    }

    #[test]
    fn decide_reindex_cases() {
        let openai = target("openai/text-embedding-3-small@1536");
        let e5 = target("local/multilingual-e5-small@384+p1");
        assert_eq!(decide_reindex(Some("openai/text-embedding-3-small"), &openai, true), ReindexDecision::Keep);
        assert_eq!(decide_reindex(Some(&openai.key()), &openai, true), ReindexDecision::Keep);
        assert_eq!(
            decide_reindex(Some("openai/text-embedding-3-small"), &e5, true),
            ReindexDecision::Reindex { from: "openai/text-embedding-3-small".into(), to: e5.clone() }
        );
        // New prefixes: a Gemini index from before them is reindexed once.
        let gemini = target("gemini/gemini-embedding-2@1536+p1");
        assert!(matches!(decide_reindex(Some("gemini/gemini-embedding-2"), &gemini, true), ReindexDecision::Reindex { .. }));
        assert_eq!(decide_reindex(None, &e5, false), ReindexDecision::Adopt);
        assert_eq!(decide_reindex(None, &openai, true), ReindexDecision::Adopt);
        assert!(matches!(decide_reindex(None, &e5, true), ReindexDecision::Reindex { .. }));
    }

    #[test]
    fn override_and_setting_choose_the_model() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_setting("active_profile", "air_gapped_power").unwrap();
        let (spec, source) = active_embedding_model(&db, None).unwrap();
        assert_eq!(source, EmbeddingModelSource::Profile(ProfileTier::AirGappedPower));
        assert_eq!(spec.id, ProfileTier::AirGappedPower.default_embedding_model());

        db.set_setting(EMBEDDING_MODEL_SETTING, "bge-m3").unwrap();
        let (spec, source) = active_embedding_model(&db, None).unwrap();
        assert_eq!((spec.id, source), ("bge-m3", EmbeddingModelSource::Setting));

        let (spec, source) = active_embedding_model(&db, Some("mock")).unwrap();
        assert!(spec.is_mock() && source == EmbeddingModelSource::Override);

        // The setting can never select the mock, and an unknown id is an error, not a fallback.
        db.set_setting(EMBEDDING_MODEL_SETTING, "mock-embed").unwrap();
        assert!(active_embedding_model(&db, None).is_err());
        db.set_setting(EMBEDDING_MODEL_SETTING, "gone-model").unwrap();
        let err = active_embedding_model(&db, None).unwrap_err();
        assert!(err.to_string().contains("gone-model") && err.fix.contains("/embedding"), "{err}");
    }

    #[test]
    fn every_profile_has_a_local_default() {
        for tier in [
            ProfileTier::Lite,
            ProfileTier::SmartBalanced,
            ProfileTier::AirGappedPower,
            ProfileTier::WorkstationPro,
            ProfileTier::SovereignUltra,
        ] {
            assert!(profile_default_model(tier).is_local(), "{tier:?}");
        }
        assert_eq!(profile_default_model(ProfileTier::Lite).id, "multilingual-e5-small");
    }

    #[test]
    fn upgrade_adopts_the_most_used_real_model_once() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        for ws in ["A", "B", "C", "D"] {
            db.create_workspace(ws, None).unwrap();
        }
        db.set_workspace_embedding_target("A", Some("gemini/gemini-embedding-2")).unwrap();
        db.set_workspace_embedding_target("B", Some("openai/text-embedding-3-small")).unwrap();
        db.set_workspace_embedding_target("C", Some("openai/text-embedding-3-small")).unwrap();
        db.set_workspace_embedding_target("D", Some("mock/mock-embed")).unwrap();
        adopt_existing_index_model(&db);
        assert_eq!(db.get_setting(EMBEDDING_MODEL_SETTING).unwrap().as_deref(), Some("text-embedding-3-small"));

        // Runs once: a later change by the user is not overwritten.
        db.set_setting(EMBEDDING_MODEL_SETTING, "multilingual-e5-base").unwrap();
        adopt_existing_index_model(&db);
        assert_eq!(db.get_setting(EMBEDDING_MODEL_SETTING).unwrap().as_deref(), Some("multilingual-e5-base"));
    }

    #[test]
    fn upgrade_ignores_mock_and_shut_down_models() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.create_workspace("A", None).unwrap();
        db.create_workspace("B", None).unwrap();
        db.set_workspace_embedding_target("A", Some("mock/mock-embed")).unwrap();
        db.set_workspace_embedding_target("B", Some("gemini/text-embedding-004")).unwrap();
        adopt_existing_index_model(&db);
        assert_eq!(db.get_setting(EMBEDDING_MODEL_SETTING).unwrap(), None);
        assert!(db.get_setting(EMBEDDING_ADOPTION_SETTING).unwrap().is_some());
    }

    #[test]
    fn cloud_model_without_key_is_unavailable_never_swapped() {
        if std::env::var("OPENAI_API_KEY").map(|v| !v.trim().is_empty()).unwrap_or(false) {
            eprintln!("skipped: OPENAI_API_KEY is set");
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        let spec = find_embedding_model("text-embedding-3-small").unwrap();
        let err = build_embedding_provider(&db, spec).err().expect("missing key is an error");
        assert_eq!(err.model, "text-embedding-3-small");
        assert!(err.cause.contains("openai") && err.fix.contains("/keys openai"), "{err}");
        db.set_api_key("openai", "sk-test").unwrap();
        assert!(build_embedding_provider(&db, spec).is_ok());
    }

    #[test]
    fn query_side_uses_the_recorded_space_or_nothing() {
        if std::env::var("GEMINI_API_KEY").map(|v| !v.trim().is_empty()).unwrap_or(false) {
            eprintln!("skipped: GEMINI_API_KEY is set");
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_setting(EMBEDDING_MODEL_SETTING, "text-embedding-3-small").unwrap();
        db.set_api_key("openai", "sk-test").unwrap();

        // Recorded Gemini index without a Gemini key: no provider, never OpenAI.
        db.set_workspace_embedding_target("Default", Some("gemini/gemini-embedding-2")).unwrap();
        let q = resolve_query_embedding(&db, "Default");
        assert!(q.provider.is_none() && q.unavailable.is_some(), "{q:?}");
        assert_eq!(q.target, target("gemini/gemini-embedding-2"));
        // An index from before the prefixes gets raw questions.
        assert_eq!(q.query_text("onde"), "onde");

        // Recorded mock index: no provider and no error (lexical only, BUG-019).
        db.set_workspace_embedding_target("Default", Some("mock/mock-embed")).unwrap();
        let q = resolve_query_embedding(&db, "Default");
        assert!(q.provider.is_none() && q.unavailable.is_none());

        // Recorded real index that can run.
        db.set_workspace_embedding_target("Default", Some("openai/text-embedding-3-small@1536")).unwrap();
        let q = resolve_query_embedding(&db, "Default");
        assert!(q.provider.is_some(), "{q:?}");
    }

    #[test]
    fn global_dense_requires_the_same_space() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        let openai = target("openai/text-embedding-3-small@1536");
        assert!(!global_dense_enabled(&db, &openai));
        db.set_setting(GLOBAL_KNOWLEDGE_TARGET_SETTING, "gemini/gemini-embedding-2@1536+p1").unwrap();
        assert!(!global_dense_enabled(&db, &openai));
        // A key recorded before ADR-0154 for the same space still counts.
        db.set_setting(GLOBAL_KNOWLEDGE_TARGET_SETTING, "openai/text-embedding-3-small").unwrap();
        assert!(global_dense_enabled(&db, &openai));
        db.set_setting(GLOBAL_KNOWLEDGE_TARGET_SETTING, "").unwrap();
        assert!(!global_dense_enabled(&db, &openai));
        db.set_setting(GLOBAL_KNOWLEDGE_TARGET_SETTING, "mock/mock-embed").unwrap();
        assert!(!global_dense_enabled(&db, &target("mock/mock-embed")));
    }

    #[test]
    fn pending_notice_and_reindexed_workspaces_follow_the_setting() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_setting(EMBEDDING_ADOPTION_SETTING, "done").unwrap();
        db.set_setting(EMBEDDING_MODEL_SETTING, "multilingual-e5-small").unwrap();
        assert_eq!(pending_reindex_notice(&db, "Default"), None);
        db.set_workspace_embedding_target("Default", Some("local/multilingual-e5-small@384+p1")).unwrap();
        assert_eq!(pending_reindex_notice(&db, "Default"), None);
        let e5 = find_embedding_model("multilingual-e5-small").unwrap().target();
        assert!(workspaces_reindexed_by(&db, &e5).is_empty());

        db.set_setting(EMBEDDING_MODEL_SETTING, "bge-m3").unwrap();
        let notice = pending_reindex_notice(&db, "Default").expect("notice");
        assert!(notice.contains("local/multilingual-e5-small@384+p1 → local/bge-m3@1024"), "{notice}");
        let m3 = find_embedding_model("bge-m3").unwrap().target();
        assert_eq!(workspaces_reindexed_by(&db, &m3), vec![("Default".to_string(), e5.key())]);
    }

    #[test]
    fn reindex_message_names_both_targets() {
        let d = ReindexDecision::Reindex {
            from: "openai/text-embedding-3-small".into(),
            to: target("local/multilingual-e5-small@384+p1"),
        };
        let msg = d.message().expect("message");
        assert!(msg.contains("openai/text-embedding-3-small → local/multilingual-e5-small@384+p1"));
        assert_eq!(ReindexDecision::Keep.message(), None);
    }

    #[test]
    fn unavailable_message_names_model_cause_and_fix() {
        let e = EmbeddingUnavailable::new("bge-m3", "download failed: offline", "Copy the files.");
        assert_eq!(e.to_string(), "Embedding model bge-m3 is unavailable: download failed: offline. Copy the files.");
    }
}
