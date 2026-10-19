//! The application-wide embedding model (ADR-0149).
//!
//! One embedding model, chosen for the whole application and independent of the chat model,
//! embeds every chunk and every query. It runs through an API (any OpenAI-compatible endpoint,
//! or Google's native Gemini format) or, later, through local ONNX weights. Every chunk records
//! the key of the model that embedded it, so a model change only re-embeds the stale files.
//! The active model is never swapped silently: when it cannot run, callers get an
//! [`EmbeddingUnavailable`] naming the model, the cause and the fix.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::storage::lancedb::NativeLanceStore;
use crate::storage::sqlite::NativeConfigDb;

use actx_lm::providers::gemini::GeminiProvider;
use actx_lm::providers::mock::MockLmProvider;
use actx_lm::providers::ProviderKind;
use actx_lm::traits::LmProvider;

/// Setting with the active embedding model, as the JSON of an [`EmbeddingConfig`].
pub const EMBEDDING_MODEL_SETTING: &str = "embedding_model";
/// Setting with every configuration still referenced by stored chunks, keyed by
/// [`EmbeddingConfig::key`], so the model of stale chunks can still be built during a migration.
pub const KNOWN_EMBEDDING_MODELS_SETTING: &str = "embedding_models_known";
/// Environment override of the active model by preset name (tests, CI).
pub const EMBEDDING_MODEL_ENV: &str = "ACTX_EMBEDDING_MODEL";

/// OpenAI embedding model of the `openai` preset.
pub const OPENAI_EMBEDDING_MODEL: &str = "text-embedding-3-small";
/// Gemini embedding model of the `gemini` preset.
pub const GEMINI_EMBEDDING_MODEL: &str = "gemini-embedding-2";
/// Ollama embedding model of the `ollama` preset.
pub const OLLAMA_EMBEDDING_MODEL: &str = "nomic-embed-text";
/// Model id of the deterministic mock provider (tests only).
pub const MOCK_EMBEDDING_MODEL: &str = "mock-embed";

const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";
const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";

/// Presets offered to the user; `mock` is accepted too but reserved for tests.
pub const EMBEDDING_PRESETS: [&str; 3] = ["openai", "gemini", "ollama"];

/// Fixed text embedded once to measure a model's vector size.
const PROBE_TEXT: &str = "AnyContext embedding dimension probe";

/// Request format of an API embedding model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiFormat {
    /// `POST <base_url>/embeddings`: OpenAI, Ollama, LM Studio, vLLM, Azure and other
    /// OpenAI-compatible servers.
    OpenAi,
    /// Google's native `batchEmbedContents`, with `outputDimensionality`.
    Gemini,
}

/// Where an embedding model runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum EmbeddingRuntime {
    /// A remote or local HTTP endpoint. `key_name` names the API key in the encrypted key
    /// store (`/keys <key_name> ...`); `None` for endpoints without a key (Ollama).
    Api {
        format: ApiFormat,
        base_url: String,
        key_name: Option<String>,
    },
    /// Local ONNX weights (Frente C). Reserved: no ONNX embedder ships yet.
    Onnx { model_path: Option<String> },
    /// Deterministic mock vectors, selected only by the `mock` preset (tests, CI).
    Mock,
}

/// The configured embedding model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub runtime: EmbeddingRuntime,
    /// Model id sent to the runtime.
    pub model: String,
    /// Vector size, measured by [`probe_dimension`] (presets carry the model's known size).
    pub dimension: usize,
}

impl EmbeddingConfig {
    /// Vector-space identity written on every chunk, e.g. `text-embedding-3-small#1536`.
    /// The endpoint is left out, so moving the same model to another URL does not reindex.
    pub fn key(&self) -> String {
        format!("{}#{}", self.model, self.dimension)
    }

    /// The mock runtime: every text gets the same vector, so its vectors carry no meaning and
    /// are never searched (BUG-019).
    pub fn is_mock(&self) -> bool {
        self.runtime == EmbeddingRuntime::Mock
    }

    /// An API model in the OpenAI-compatible format.
    pub fn openai_compatible(base_url: &str, model: &str, key_name: Option<&str>, dimension: usize) -> Self {
        Self::api(ApiFormat::OpenAi, base_url, model, key_name, dimension)
    }

    fn api(format: ApiFormat, base_url: &str, model: &str, key_name: Option<&str>, dimension: usize) -> Self {
        Self {
            runtime: EmbeddingRuntime::Api {
                format,
                base_url: base_url.trim_end_matches('/').to_string(),
                key_name: key_name.map(|k| k.trim().to_lowercase()).filter(|k| !k.is_empty()),
            },
            model: model.trim().to_string(),
            dimension,
        }
    }

    /// Builds the configuration of a preset (`openai`, `gemini`, `ollama`, or `mock` for tests).
    pub fn preset(name: &str) -> Option<Self> {
        match name.trim().to_lowercase().as_str() {
            "openai" => Some(Self::api(ApiFormat::OpenAi, OPENAI_BASE_URL, OPENAI_EMBEDDING_MODEL, Some("openai"), 1536)),
            "gemini" => Some(Self::api(ApiFormat::Gemini, GEMINI_BASE_URL, GEMINI_EMBEDDING_MODEL, Some("gemini"), 1536)),
            "ollama" => Some(Self::api(ApiFormat::OpenAi, OLLAMA_BASE_URL, OLLAMA_EMBEDDING_MODEL, None, 768)),
            m if m.starts_with("mock") => Some(Self {
                runtime: EmbeddingRuntime::Mock,
                model: MOCK_EMBEDDING_MODEL.to_string(),
                dimension: 1536,
            }),
            _ => None,
        }
    }

    /// One-line description for status output, e.g.
    /// `text-embedding-3-small (1536 dimensions, openai format at https://api.openai.com/v1)`.
    pub fn describe(&self) -> String {
        let runtime = match &self.runtime {
            EmbeddingRuntime::Api { format, base_url, .. } => {
                let format = match format {
                    ApiFormat::OpenAi => "openai",
                    ApiFormat::Gemini => "gemini",
                };
                format!("{format} format at {base_url}")
            }
            EmbeddingRuntime::Onnx { .. } => "local ONNX".to_string(),
            EmbeddingRuntime::Mock => "mock vectors".to_string(),
        };
        format!("{} ({} dimensions, {runtime})", self.model, self.dimension)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

/// The active embedding model cannot run: missing key, unknown preset, unsupported runtime or a
/// failed probe. Shown to the user as is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingUnavailable {
    /// The model (or preset) that cannot run.
    pub model: String,
    pub reason: String,
    pub fix: String,
}

impl EmbeddingUnavailable {
    fn new(model: impl Into<String>, reason: impl Into<String>, fix: impl Into<String>) -> Self {
        Self { model: model.into(), reason: reason.into(), fix: fix.into() }
    }
}

impl fmt::Display for EmbeddingUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Embedding model {} cannot run: {}. Fix: {}", self.model, self.reason, self.fix)
    }
}

impl std::error::Error for EmbeddingUnavailable {}

/// The configuration of a preset name, or the error that names the valid presets.
pub fn preset_config(name: &str) -> Result<EmbeddingConfig, EmbeddingUnavailable> {
    EmbeddingConfig::preset(name).ok_or_else(|| {
        EmbeddingUnavailable::new(
            name.trim(),
            "unknown embedding preset",
            format!("use one of {}, or configure an endpoint with /embed model --url", EMBEDDING_PRESETS.join(", ")),
        )
    })
}

/// The active embedding model, without building its provider.
///
/// Order: `preset_override` (a sync option used by tests), `ACTX_EMBEDDING_MODEL`, the global
/// setting. Without a setting, the model is chosen once from the existing index (or the default
/// chat model on a new installation) and stored ([`crate::ingestion::embedding_migration`]).
pub fn active_embedding_config(
    db: &NativeConfigDb,
    preset_override: Option<&str>,
) -> Result<EmbeddingConfig, EmbeddingUnavailable> {
    let env_override = std::env::var(EMBEDDING_MODEL_ENV).ok().filter(|v| !v.trim().is_empty());
    if let Some(name) = preset_override.map(str::to_string).or(env_override) {
        return preset_config(&name);
    }
    if let Some(config) = stored_embedding_config(db) {
        return Ok(config);
    }
    Ok(crate::ingestion::embedding_migration::adopt_global_model(db))
}

/// The configuration stored in the global setting, if any.
pub fn stored_embedding_config(db: &NativeConfigDb) -> Option<EmbeddingConfig> {
    db.get_setting(EMBEDDING_MODEL_SETTING)
        .ok()
        .flatten()
        .and_then(|json| EmbeddingConfig::from_json(&json))
}

/// Stores `config` as the active model and as a known configuration.
pub fn set_active_embedding_config(db: &NativeConfigDb, config: &EmbeddingConfig) -> Result<(), String> {
    remember_embedding_config(db, config)?;
    db.set_setting(EMBEDDING_MODEL_SETTING, &config.to_json()).map_err(|e| e.to_string())
}

/// Every configuration referenced by stored chunks, keyed by [`EmbeddingConfig::key`].
pub fn known_embedding_configs(db: &NativeConfigDb) -> BTreeMap<String, EmbeddingConfig> {
    db.get_setting(KNOWN_EMBEDDING_MODELS_SETTING)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

/// Adds `config` to the known configurations (replacing one with the same key).
pub fn remember_embedding_config(db: &NativeConfigDb, config: &EmbeddingConfig) -> Result<(), String> {
    let mut known = known_embedding_configs(db);
    if known.get(&config.key()) == Some(config) {
        return Ok(());
    }
    known.insert(config.key(), config.clone());
    let json = serde_json::to_string(&known).map_err(|e| e.to_string())?;
    db.set_setting(KNOWN_EMBEDDING_MODELS_SETTING, &json).map_err(|e| e.to_string())
}

/// Builds the provider of a configuration, or says why it cannot run. No network call is made:
/// an unreachable endpoint shows up on the first embedding call.
pub fn provider_for(db: &NativeConfigDb, config: &EmbeddingConfig) -> Result<Arc<dyn LmProvider>, EmbeddingUnavailable> {
    match &config.runtime {
        EmbeddingRuntime::Mock => Ok(Arc::new(MockLmProvider::new())),
        EmbeddingRuntime::Onnx { .. } => Err(EmbeddingUnavailable::new(
            &config.model,
            "local ONNX embedding models are not available yet",
            "choose an API model with /embed model",
        )),
        EmbeddingRuntime::Api { format, base_url, key_name } => {
            let key = match key_name {
                Some(name) => Some(db.get_api_key(name).ok().flatten().ok_or_else(|| {
                    EmbeddingUnavailable::new(
                        &config.model,
                        format!("no API key named '{name}'"),
                        format!("add it with /keys {name} <api_key>, or choose another model with /embed model"),
                    )
                })?),
                None => None,
            };
            let built = match format {
                ApiFormat::OpenAi => ProviderKind::CustomCompatible {
                    name: key_name.clone().unwrap_or_else(|| "openai-compatible".to_string()),
                    base_url: base_url.clone(),
                }
                .build(key, None),
                ApiFormat::Gemini => match key {
                    Some(key) => Ok(Arc::new(
                        GeminiProvider::new(key)
                            .with_base_url(base_url.clone())
                            .with_output_dimensionality(config.dimension),
                    ) as Arc<dyn LmProvider>),
                    None => Err(actx_lm::LmError::Auth {
                        provider: "gemini".to_string(),
                        message: "the gemini format needs an API key".to_string(),
                    }),
                },
            };
            built.map_err(|e| {
                EmbeddingUnavailable::new(&config.model, e.to_string(), "check the endpoint and key with /embed")
            })
        }
    }
}

/// The active model and its provider. Ingestion (chunk vectors) and retrieval (query vectors)
/// both use it, so a query is embedded in the same vector space as the index (ADR-0132).
pub fn resolve_active_embedding(
    db: &NativeConfigDb,
) -> Result<(Arc<dyn LmProvider>, EmbeddingConfig), EmbeddingUnavailable> {
    resolve_embedding(db, None)
}

/// [`resolve_active_embedding`] with an optional preset override (`SyncOptions::model`).
pub fn resolve_embedding(
    db: &NativeConfigDb,
    preset_override: Option<&str>,
) -> Result<(Arc<dyn LmProvider>, EmbeddingConfig), EmbeddingUnavailable> {
    let config = active_embedding_config(db, preset_override)?;
    let provider = provider_for(db, &config)?;
    Ok((provider, config))
}

/// Measures a model's vector size with one embedding call on a fixed text. The `gemini` format
/// asks for `config.dimension` values (`outputDimensionality`), so it returns that size.
pub async fn probe_dimension(
    provider: &Arc<dyn LmProvider>,
    config: &EmbeddingConfig,
) -> Result<usize, EmbeddingUnavailable> {
    let fix = "check the endpoint, the model name and the key";
    let vectors = provider
        .embed(&config.model, &[PROBE_TEXT.to_string()])
        .await
        .map_err(|e| EmbeddingUnavailable::new(&config.model, format!("the probe embedding failed: {e}"), fix))?;
    match vectors.first().map(Vec::len) {
        Some(n) if n > 0 => Ok(n),
        _ => Err(EmbeddingUnavailable::new(&config.model, "the probe embedding returned no vector", fix)),
    }
}

/// Whether dense search may include the `Global` workspace for queries embedded with `config`:
/// only once its rows were embedded with that model, never on the mock runtime (BUG-019). The
/// placeholders written by the startup bootstrap record no model.
pub fn global_dense_enabled(lance_store: &NativeLanceStore, config: &EmbeddingConfig) -> bool {
    if config.is_mock() {
        return false;
    }
    let filter = format!("workspace = 'Global' AND embedding_model = '{}'", config.key().replace('\'', "''"));
    lance_store.count_rows_where(&filter, None).map(|n| n > 0).unwrap_or(false)
}

/// Notice for the command layer when files of a workspace were embedded with another model than
/// the active one, so the next sync re-embeds them.
pub fn stale_files_notice(db: &NativeConfigDb, workspace: &str) -> Option<String> {
    let active = active_embedding_config(db, None).ok()?;
    let stale = db.count_stale_files(workspace, &active.key()).ok()?;
    (stale > 0).then(|| {
        let noun = if stale == 1 { "file was" } else { "files were" };
        format!("{stale} {noun} embedded with another model; this sync re-embeds them with {}.", active.key())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The resolver reads provider keys and the model override from the environment; these
    /// tests need a clean one and are skipped otherwise.
    pub(crate) fn env_is_clean() -> bool {
        let vars = ["OPENAI_API_KEY", "GEMINI_API_KEY", "GOOGLE_API_KEY", EMBEDDING_MODEL_ENV];
        let clean = vars.iter().all(|v| std::env::var(v).map(|s| s.trim().is_empty()).unwrap_or(true));
        if !clean {
            eprintln!("skipped: one of {vars:?} is set");
        }
        clean
    }

    #[test]
    fn presets_fill_endpoint_format_model_and_key() {
        let openai = EmbeddingConfig::preset("openai").expect("openai");
        assert_eq!(
            openai.runtime,
            EmbeddingRuntime::Api { format: ApiFormat::OpenAi, base_url: OPENAI_BASE_URL.into(), key_name: Some("openai".into()) }
        );
        assert_eq!(openai.key(), "text-embedding-3-small#1536");

        let gemini = EmbeddingConfig::preset("Gemini").expect("gemini");
        assert!(matches!(gemini.runtime, EmbeddingRuntime::Api { format: ApiFormat::Gemini, ref key_name, .. } if key_name.as_deref() == Some("gemini")));
        assert_eq!(gemini.key(), "gemini-embedding-2#1536");

        let ollama = EmbeddingConfig::preset("ollama").expect("ollama");
        assert!(matches!(ollama.runtime, EmbeddingRuntime::Api { format: ApiFormat::OpenAi, key_name: None, .. }));
        assert_eq!(ollama.key(), "nomic-embed-text#768");

        assert!(EmbeddingConfig::preset("mock").expect("mock").is_mock());
        assert_eq!(EmbeddingConfig::preset("voyage"), None);
        let err = preset_config("voyage").unwrap_err();
        assert!(err.to_string().contains("openai, gemini, ollama"), "{err}");
    }

    #[test]
    fn key_ignores_the_endpoint() {
        let a = EmbeddingConfig::openai_compatible("http://localhost:11434/v1", "nomic-embed-text", None, 768);
        let b = EmbeddingConfig::openai_compatible("http://gpu-box:8080/v1/", "nomic-embed-text", Some("lab"), 768);
        assert_eq!(a.key(), b.key());
        assert_ne!(a, b);
        assert_eq!(b.runtime, EmbeddingRuntime::Api {
            format: ApiFormat::OpenAi,
            base_url: "http://gpu-box:8080/v1".into(),
            key_name: Some("lab".into()),
        });
    }

    #[test]
    fn config_round_trips_through_json() {
        for config in [
            EmbeddingConfig::preset("gemini").unwrap(),
            EmbeddingConfig::preset("mock").unwrap(),
            EmbeddingConfig { runtime: EmbeddingRuntime::Onnx { model_path: None }, model: "bge-small".into(), dimension: 384 },
        ] {
            assert_eq!(EmbeddingConfig::from_json(&config.to_json()), Some(config));
        }
        assert_eq!(EmbeddingConfig::from_json("openai/text-embedding-3-small"), None);
    }

    #[test]
    fn known_configs_accumulate_by_key() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        assert!(known_embedding_configs(&db).is_empty());
        set_active_embedding_config(&db, &EmbeddingConfig::preset("openai").unwrap()).unwrap();
        set_active_embedding_config(&db, &EmbeddingConfig::preset("ollama").unwrap()).unwrap();
        let known = known_embedding_configs(&db);
        assert_eq!(known.keys().cloned().collect::<Vec<_>>(), vec!["nomic-embed-text#768", "text-embedding-3-small#1536"]);
        assert_eq!(stored_embedding_config(&db), EmbeddingConfig::preset("ollama"));
    }

    #[test]
    fn the_stored_model_is_independent_of_the_chat_model() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        set_active_embedding_config(&db, &EmbeddingConfig::preset("openai").unwrap()).unwrap();
        db.set_default_model("gemini-2.5-flash").unwrap();
        db.set_workspace_model("Default", "ollama/llama3").unwrap();
        assert_eq!(active_embedding_config(&db, None).unwrap().key(), "text-embedding-3-small#1536");
        // A sync option overrides it by preset name.
        assert!(active_embedding_config(&db, Some("mock")).unwrap().is_mock());
    }

    /// Rule 4 (ADR-0149): without the key of the active model there is no fallback to another
    /// provider or to the mock vectors; the error names the model and the fix.
    #[test]
    fn a_missing_key_is_an_error_without_fallback() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        db.set_api_key("openai", "sk-test").unwrap();
        set_active_embedding_config(&db, &EmbeddingConfig::preset("gemini").unwrap()).unwrap();
        let err = resolve_active_embedding(&db).err().expect("no gemini key");
        assert_eq!(err.model, "gemini-embedding-2");
        assert!(err.reason.contains("no API key named 'gemini'"), "{err}");
        assert!(err.fix.contains("/keys gemini"), "{err}");

        db.set_api_key("gemini", "gm-test").unwrap();
        let (provider, config) = resolve_active_embedding(&db).expect("gemini key");
        assert_eq!(provider.provider_id(), "gemini");
        assert_eq!(config.key(), "gemini-embedding-2#1536");
    }

    #[test]
    fn endpoints_without_a_key_and_onnx() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        let lm_studio = EmbeddingConfig::openai_compatible("http://localhost:1234/v1", "text-embedding-nomic", None, 768);
        assert!(provider_for(&db, &lm_studio).is_ok());
        let onnx = EmbeddingConfig { runtime: EmbeddingRuntime::Onnx { model_path: None }, model: "bge-small".into(), dimension: 384 };
        let err = provider_for(&db, &onnx).err().expect("onnx is reserved");
        assert!(err.reason.contains("not available yet"), "{err}");
    }

    /// Answers an OpenAI-compatible `/embeddings` call with a 3-value vector.
    fn serve_embedding(dimension: usize) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0u8; 16384];
            let n = stream.read(&mut buf).unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let vector = vec!["0.25"; dimension].join(",");
            let body = format!(r#"{{"object":"list","data":[{{"object":"embedding","index":0,"embedding":[{vector}]}}],"model":"m"}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            request
        });
        (base, handle)
    }

    /// Rule 8 (ADR-0149): a custom OpenAI-compatible endpoint goes through the same code path
    /// as the presets, and its dimension is measured, not catalogued.
    #[tokio::test]
    async fn probe_measures_a_custom_endpoint() {
        let (base, server) = serve_embedding(3);
        let db = NativeConfigDb::open_in_memory().expect("db");
        let config = EmbeddingConfig::openai_compatible(&base, "lab-embedder", None, 0);
        let provider = provider_for(&db, &config).expect("provider");
        assert_eq!(probe_dimension(&provider, &config).await, Ok(3));
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v1/embeddings"), "{request}");
        assert!(request.contains("lab-embedder"), "{request}");
    }

    #[tokio::test]
    async fn a_failed_probe_reports_the_cause() {
        let db = NativeConfigDb::open_in_memory().expect("db");
        // Nothing listens on the discard port.
        let config = EmbeddingConfig::openai_compatible("http://127.0.0.1:9/v1", "lab-embedder", None, 0);
        let provider = provider_for(&db, &config).expect("provider");
        let err = probe_dimension(&provider, &config).await.unwrap_err();
        assert_eq!(err.model, "lab-embedder");
        assert!(err.reason.starts_with("the probe embedding failed"), "{err}");
    }

    #[test]
    fn stale_notice_counts_files_of_other_models() {
        if !env_is_clean() {
            return;
        }
        let db = NativeConfigDb::open_in_memory().expect("db");
        set_active_embedding_config(&db, &EmbeddingConfig::preset("openai").unwrap()).unwrap();
        assert_eq!(stale_files_notice(&db, "Default"), None);
        db.upsert_file_stat_cache("Default", "/docs/a.md", 1.0, 10, Some("h1"), Some("text-embedding-3-small#1536")).unwrap();
        assert_eq!(stale_files_notice(&db, "Default"), None);
        db.upsert_file_stat_cache("Default", "/docs/b.md", 1.0, 10, Some("h2"), Some("nomic-embed-text#768")).unwrap();
        let notice = stale_files_notice(&db, "Default").expect("notice");
        assert!(notice.starts_with("1 file was embedded with another model"), "{notice}");
    }
}
