//! Embedding target resolution and the reindex rule (ADR-0138).
//!
//! A workspace's vectors are only comparable with query vectors from the same provider and
//! model. The sync records the target that built a workspace's vectors and reindexes the whole
//! workspace when the resolved target changes; queries prefer the recorded target until then.

use std::sync::Arc;

use crate::storage::sqlite::NativeConfigDb;

use actx_lm::providers::gemini::GeminiProvider;
use actx_lm::providers::mock::MockLmProvider;
use actx_lm::providers::openai::OpenAiCompatibleProvider;
use actx_lm::traits::LmProvider;

/// Gemini embedding model (successor of the shut-down `text-embedding-004`).
pub const GEMINI_EMBEDDING_MODEL: &str = "gemini-embedding-2";
/// Gemini embedding model used before v0.45.1, shut down by Google on 2026-01-14.
pub const LEGACY_GEMINI_EMBEDDING_MODEL: &str = "text-embedding-004";
/// OpenAI embedding model.
pub const OPENAI_EMBEDDING_MODEL: &str = "text-embedding-3-small";
/// Ollama embedding model.
pub const OLLAMA_EMBEDDING_MODEL: &str = "nomic-embed-text";
/// Model id of the deterministic mock provider (no API key, or an explicit `mock*` model).
pub const MOCK_EMBEDDING_MODEL: &str = "mock-embed";

const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";
const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// The provider and model that produce a workspace's vectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingTarget {
    /// `openai`, `gemini`, `ollama` or `mock`.
    pub provider: String,
    /// Model id sent to the provider.
    pub model: String,
}

impl EmbeddingTarget {
    pub fn new(provider: &str, model: &str) -> Self {
        Self {
            provider: provider.to_string(),
            model: model.to_string(),
        }
    }

    /// The deterministic mock target: every text gets the same vector, so its vectors carry no
    /// meaning and are never searched (BUG-019).
    pub fn is_mock(&self) -> bool {
        self.provider == "mock"
    }

    /// Stable text form stored in SQLite, e.g. `openai/text-embedding-3-small`.
    pub fn key(&self) -> String {
        format!("{}/{}", self.provider, self.model)
    }

    /// Parses the text form written by [`EmbeddingTarget::key`].
    pub fn parse(key: &str) -> Option<Self> {
        let (provider, model) = key.trim().split_once('/')?;
        if provider.is_empty() || model.is_empty() {
            return None;
        }
        Some(Self::new(provider, model))
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

/// Resolves the embedding provider and target for a workspace.
///
/// Ingestion (chunk vectors) and retrieval (query vectors) must both use this resolution so a
/// query is embedded in the same vector space as the index (ADR-0132).
/// Order: explicit override, `ACTX_EMBEDDING_MODEL`, `ACTX_MODEL`, workspace model, default model.
pub fn resolve_embedding_provider(
    db: &NativeConfigDb,
    model_override: Option<&str>,
    workspace: &str,
) -> (Option<Arc<dyn LmProvider>>, EmbeddingTarget) {
    let model = model_override
        .map(|s| s.to_string())
        .or_else(|| std::env::var("ACTX_EMBEDDING_MODEL").ok())
        .or_else(|| std::env::var("ACTX_MODEL").ok())
        .or_else(|| db.get_workspace_model(workspace).ok())
        .or_else(|| db.get_default_model().ok())
        .unwrap_or_else(|| OPENAI_EMBEDDING_MODEL.to_string());

    let raw = model.trim().to_lowercase();

    let preferred = if raw.starts_with("mock") {
        EmbeddingTarget::new("mock", MOCK_EMBEDDING_MODEL)
    } else if raw.starts_with("gemini") {
        EmbeddingTarget::new("gemini", GEMINI_EMBEDDING_MODEL)
    } else if raw.starts_with("ollama") || raw.starts_with("local") {
        EmbeddingTarget::new("ollama", OLLAMA_EMBEDDING_MODEL)
    } else {
        EmbeddingTarget::new("openai", OPENAI_EMBEDDING_MODEL)
    };

    if let Some(provider) = build_provider(db, &preferred) {
        return (Some(provider), preferred);
    }

    // Missing key for the preferred provider: OpenAI, then the deterministic mock provider.
    let openai = EmbeddingTarget::new("openai", OPENAI_EMBEDDING_MODEL);
    if let Some(provider) = build_provider(db, &openai) {
        return (Some(provider), openai);
    }
    let mock = EmbeddingTarget::new("mock", MOCK_EMBEDDING_MODEL);
    (Some(Arc::new(MockLmProvider::new())), mock)
}

/// Provider and target for query vectors.
///
/// Prefers the target recorded by the last completed sync, so a model change does not break
/// dense search before the next sync reindexes the workspace. Falls back to the current
/// resolution when nothing is recorded or the recorded provider cannot be built.
/// The mock target gets no provider: its vectors are all the same, so the query is answered by
/// BM25 only (BUG-019).
pub fn resolve_query_embedding_provider(
    db: &NativeConfigDb,
    workspace: &str,
) -> (Option<Arc<dyn LmProvider>>, EmbeddingTarget) {
    let current = resolve_embedding_provider(db, None, workspace);
    let recorded = db
        .get_workspace_embedding_target(workspace)
        .ok()
        .flatten()
        .flatten()
        .and_then(|key| EmbeddingTarget::parse(&key));
    let (provider, target) = match recorded {
        Some(target) if target != current.1 => match build_provider(db, &target) {
            Some(provider) => (Some(provider), target),
            None => current,
        },
        _ => current,
    };
    if target.is_mock() {
        (None, target)
    } else {
        (provider, target)
    }
}

/// Notice for the command layer when the next sync of a workspace will reindex it, e.g. right
/// after `/model` changed the chat model. Uses the SQLite stat cache as the "indexed" signal.
pub fn pending_reindex_notice(db: &NativeConfigDb, workspace: &str) -> Option<String> {
    let recorded = db.get_workspace_embedding_target(workspace).ok().flatten()?;
    let (_, current) = resolve_embedding_provider(db, None, workspace);
    let has_indexed_data = !db.get_workspace_files_stat_cache(workspace).unwrap_or_default().is_empty();
    match decide_reindex(recorded.as_deref(), &current, has_indexed_data) {
        ReindexDecision::Reindex { from, to } => Some(format!(
            "Embedding model changes ({} → {}): the next sync reindexes the whole workspace.",
            from,
            to.key()
        )),
        _ => None,
    }
}

/// Target that built the vectors of a workspace indexed before the target was recorded.
///
/// The pre-v0.45.1 resolver gave the same result for the same inputs, except that Gemini
/// resolved to `text-embedding-004`.
pub fn legacy_target(current: &EmbeddingTarget) -> EmbeddingTarget {
    if current.provider == "gemini" {
        EmbeddingTarget::new("gemini", LEGACY_GEMINI_EMBEDDING_MODEL)
    } else {
        current.clone()
    }
}

/// Decides whether the stored vectors of a workspace can be kept.
///
/// `recorded` is the stored target key (`None` for a workspace indexed before the record
/// existed) and `has_indexed_data` whether the workspace has any stored chunk.
pub fn decide_reindex(
    recorded: Option<&str>,
    current: &EmbeddingTarget,
    has_indexed_data: bool,
) -> ReindexDecision {
    match recorded {
        Some(key) if key == current.key() => ReindexDecision::Keep,
        Some(key) => ReindexDecision::Reindex {
            from: key.to_string(),
            to: current.clone(),
        },
        None if !has_indexed_data => ReindexDecision::Adopt,
        None => {
            let legacy = legacy_target(current);
            if legacy == *current {
                ReindexDecision::Adopt
            } else {
                ReindexDecision::Reindex {
                    from: legacy.key(),
                    to: current.clone(),
                }
            }
        }
    }
}

/// Builds the provider for a target, or `None` when its API key is missing or the model can no
/// longer be served.
fn build_provider(db: &NativeConfigDb, target: &EmbeddingTarget) -> Option<Arc<dyn LmProvider>> {
    match target.provider.as_str() {
        "mock" => Some(Arc::new(MockLmProvider::new())),
        "ollama" => Some(Arc::new(OpenAiCompatibleProvider::new(
            "ollama",
            OLLAMA_BASE_URL.to_string(),
            None,
        ))),
        "gemini" if target.model != LEGACY_GEMINI_EMBEDDING_MODEL => {
            let key = db.get_api_key("gemini").ok().flatten()?;
            Some(Arc::new(GeminiProvider::new(key)))
        }
        "openai" => {
            let key = db.get_api_key("openai").ok().flatten()?;
            Some(Arc::new(OpenAiCompatibleProvider::new(
                "openai",
                OPENAI_BASE_URL.to_string(),
                Some(key),
            )))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(key: &str) -> EmbeddingTarget {
        EmbeddingTarget::parse(key).expect("valid key")
    }

    #[test]
    fn key_round_trips() {
        let t = EmbeddingTarget::new("openai", OPENAI_EMBEDDING_MODEL);
        assert_eq!(t.key(), "openai/text-embedding-3-small");
        assert_eq!(EmbeddingTarget::parse(&t.key()), Some(t));
        assert_eq!(EmbeddingTarget::parse("no-separator"), None);
        assert_eq!(EmbeddingTarget::parse("/model"), None);
    }

    #[test]
    fn decide_reindex_cases() {
        let openai = target("openai/text-embedding-3-small");
        let gemini = target("gemini/gemini-embedding-2");

        // Recorded and equal.
        assert_eq!(decide_reindex(Some("openai/text-embedding-3-small"), &openai, true), ReindexDecision::Keep);
        // Recorded and different.
        assert_eq!(
            decide_reindex(Some("openai/text-embedding-3-small"), &gemini, true),
            ReindexDecision::Reindex { from: "openai/text-embedding-3-small".into(), to: gemini.clone() }
        );
        // Empty workspace without a record.
        assert_eq!(decide_reindex(None, &gemini, false), ReindexDecision::Adopt);
        // Workspace from before the record, same target as the legacy resolver.
        assert_eq!(decide_reindex(None, &openai, true), ReindexDecision::Adopt);
        assert_eq!(decide_reindex(None, &target("mock/mock-embed"), true), ReindexDecision::Adopt);
    }

    /// Regression BUG-013: a Gemini workspace from before the fix holds `text-embedding-004`
    /// vectors (or pseudo-vectors after the shutdown) and is reindexed on its first sync.
    #[test]
    fn legacy_gemini_workspace_is_reindexed() {
        let gemini = target("gemini/gemini-embedding-2");
        assert_eq!(legacy_target(&gemini), target("gemini/text-embedding-004"));
        assert_eq!(
            decide_reindex(None, &gemini, true),
            ReindexDecision::Reindex { from: "gemini/text-embedding-004".into(), to: gemini }
        );
    }

    /// The resolver reads provider keys and model overrides from the environment; these tests
    /// need a clean one and are skipped otherwise.
    fn env_is_clean() -> bool {
        let vars = ["OPENAI_API_KEY", "GEMINI_API_KEY", "ACTX_EMBEDDING_MODEL", "ACTX_MODEL"];
        let clean = vars.iter().all(|v| std::env::var(v).map(|s| s.trim().is_empty()).unwrap_or(true));
        if !clean {
            eprintln!("skipped: one of {vars:?} is set");
        }
        clean
    }

    fn resolved(db: &NativeConfigDb, model: &str) -> String {
        resolve_embedding_provider(db, Some(model), "Default").1.key()
    }

    #[test]
    fn resolver_targets_follow_model_and_keys() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        // No key: the mock provider is reported as such, not as text-embedding-3-small.
        assert_eq!(resolved(&db, "gpt-4o-mini"), "mock/mock-embed");
        assert_eq!(resolved(&db, "gemini-2.5-flash"), "mock/mock-embed");
        assert_eq!(resolved(&db, "mock"), "mock/mock-embed");
        assert_eq!(resolved(&db, "ollama/llama3"), "ollama/nomic-embed-text");

        db.set_api_key("openai", "sk-test").expect("key");
        assert_eq!(resolved(&db, "gpt-4o-mini"), "openai/text-embedding-3-small");
        // Gemini model without a Gemini key falls back to OpenAI.
        assert_eq!(resolved(&db, "gemini-2.5-flash"), "openai/text-embedding-3-small");

        db.set_api_key("gemini", "gm-test").expect("key");
        assert_eq!(resolved(&db, "gemini-2.5-flash"), "gemini/gemini-embedding-2");
    }

    #[test]
    fn query_side_prefers_the_recorded_target() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_api_key("openai", "sk-test").expect("key");
        db.set_api_key("gemini", "gm-test").expect("key");
        db.set_workspace_model("Default", "gemini-2.5-flash").expect("model");

        // Nothing recorded: the current resolution.
        assert_eq!(resolve_query_embedding_provider(&db, "Default").1.key(), "gemini/gemini-embedding-2");

        // Indexed with OpenAI, chat model changed since: queries stay in the OpenAI space.
        db.set_workspace_embedding_target("Default", Some("openai/text-embedding-3-small")).expect("record");
        assert_eq!(resolve_query_embedding_provider(&db, "Default").1.key(), "openai/text-embedding-3-small");

        // The shut-down Gemini model can't serve queries: the current target is used.
        db.set_workspace_embedding_target("Default", Some("gemini/text-embedding-004")).expect("record");
        assert_eq!(resolve_query_embedding_provider(&db, "Default").1.key(), "gemini/gemini-embedding-2");
    }

    #[test]
    fn query_side_falls_back_when_the_recorded_provider_has_no_key() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_workspace_model("Default", "mock").expect("model");
        db.set_workspace_embedding_target("Default", Some("openai/text-embedding-3-small")).expect("record");
        assert_eq!(resolve_query_embedding_provider(&db, "Default").1.key(), "mock/mock-embed");
    }

    /// Regression BUG-019: mock vectors are all the same, so queries on the mock target get no
    /// provider and skip dense search; real targets keep theirs.
    #[test]
    fn query_provider_is_none_on_the_mock_target() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        // No key: the fallback target.
        let (provider, target) = resolve_query_embedding_provider(&db, "Default");
        assert!(target.is_mock() && provider.is_none(), "{}", target.key());
        // Explicit mock model.
        db.set_workspace_model("Default", "mock").expect("model");
        let (provider, target) = resolve_query_embedding_provider(&db, "Default");
        assert!(target.is_mock() && provider.is_none(), "{}", target.key());
        // Recorded mock target after a key was added: the recorded target is still mock.
        db.set_api_key("openai", "sk-test").expect("key");
        db.set_workspace_model("Default", "gpt-4o-mini").expect("model");
        db.set_workspace_embedding_target("Default", Some("mock/mock-embed")).expect("record");
        let (provider, target) = resolve_query_embedding_provider(&db, "Default");
        assert!(target.is_mock() && provider.is_none(), "{}", target.key());
        // Real target.
        db.set_workspace_embedding_target("Default", Some("openai/text-embedding-3-small")).expect("record");
        let (provider, target) = resolve_query_embedding_provider(&db, "Default");
        assert_eq!(target.key(), "openai/text-embedding-3-small");
        assert!(provider.is_some());
        // Ingestion still gets the mock provider: the rows are needed, the vectors are never searched.
        db.set_workspace_model("Default", "mock").expect("model");
        assert!(resolve_embedding_provider(&db, None, "Default").0.is_some());
    }

    #[test]
    fn pending_notice_only_when_the_next_sync_reindexes() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_workspace_model("Default", "mock").expect("model");
        // Nothing recorded and nothing indexed.
        assert_eq!(pending_reindex_notice(&db, "Default"), None);
        // Unknown workspace.
        assert_eq!(pending_reindex_notice(&db, "Missing"), None);

        db.set_workspace_embedding_target("Default", Some("mock/mock-embed")).expect("record");
        assert_eq!(pending_reindex_notice(&db, "Default"), None);

        db.set_workspace_embedding_target("Default", Some("openai/text-embedding-3-small")).expect("record");
        let notice = pending_reindex_notice(&db, "Default").expect("notice");
        assert!(notice.contains("openai/text-embedding-3-small → mock/mock-embed"), "{notice}");
    }

    #[test]
    fn gemini_output_size_matches_the_vector_table() {
        assert_eq!(
            actx_lm::providers::gemini::GEMINI_EMBEDDING_DIM,
            crate::storage::lancedb::DEFAULT_VECTOR_DIM
        );
    }

    #[test]
    fn reindex_message_names_both_targets() {
        let d = ReindexDecision::Reindex {
            from: "openai/text-embedding-3-small".into(),
            to: target("gemini/gemini-embedding-2"),
        };
        let msg = d.message().expect("message");
        assert!(msg.contains("openai/text-embedding-3-small → gemini/gemini-embedding-2"));
        assert_eq!(ReindexDecision::Keep.message(), None);
    }
}
