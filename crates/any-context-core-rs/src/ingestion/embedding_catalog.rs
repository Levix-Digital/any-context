//! Catalog of embedding models (FEAT-067, IMP-029, ADR-0154).
//!
//! Every embedding model AnyContext can use, local or cloud, with what the pipeline needs to
//! embed text in the model's own vector space: dimension, input limit, pooling and the query and
//! document prefixes the model was trained with. Local entries name their verified ONNX files
//! in the model manager catalog (same id).

use crate::ingestion::embedding::EmbeddingTarget;

/// Where an embedding model runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingRuntime {
    /// ONNX weights on this machine, from the model manager catalog.
    Local,
    /// A cloud or local HTTP provider (`openai`, `gemini` or `ollama`); document text leaves the
    /// process.
    Cloud(&'static str),
    /// Deterministic placeholder vectors. Never chosen by the product: tests and the benchmark's
    /// lexical-only run request it explicitly.
    Mock,
}

/// How a local model turns token states into one vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pooling {
    /// Mean of the token states under the attention mask (e5).
    Mean,
    /// State of the first token (BGE-M3).
    Cls,
}

/// What a text is embedded for: the model may expect a different prefix for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedRole {
    Query,
    Document,
}

/// One embedding model of the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddingModelSpec {
    /// Catalog id, also the model id sent to a cloud provider and the model manager id of a
    /// local model.
    pub id: &'static str,
    pub name: &'static str,
    pub runtime: EmbeddingRuntime,
    pub license: &'static str,
    pub dimension: usize,
    /// Input tokens the model reads; longer inputs are truncated.
    pub max_tokens: usize,
    /// Local models only.
    pub pooling: Pooling,
    pub query_prefix: &'static str,
    pub document_prefix: &'static str,
    /// Revision of the prefixes, part of the vector-space identity: bumped whenever a prefix
    /// changes, so workspaces embedded with the previous prefixes are reindexed. 0 = no prefixes.
    pub prefix_rev: u32,
}

impl EmbeddingModelSpec {
    /// The vector-space identity of this model with its current prefixes.
    pub fn target(&self) -> EmbeddingTarget {
        EmbeddingTarget::new(self.provider(), self.id, self.dimension, self.prefix_rev)
    }

    /// Provider part of the target: `local`, the cloud provider or `mock`.
    pub fn provider(&self) -> &'static str {
        match self.runtime {
            EmbeddingRuntime::Local => "local",
            EmbeddingRuntime::Cloud(provider) => provider,
            EmbeddingRuntime::Mock => "mock",
        }
    }

    pub fn is_local(&self) -> bool {
        self.runtime == EmbeddingRuntime::Local
    }

    pub fn is_mock(&self) -> bool {
        self.runtime == EmbeddingRuntime::Mock
    }

    /// The prefix for `role`, or `""`.
    pub fn prefix(&self, role: EmbedRole) -> &'static str {
        match role {
            EmbedRole::Query => self.query_prefix,
            EmbedRole::Document => self.document_prefix,
        }
    }
}

/// Text actually sent to the model: the prefix of `role` for the vector space of `target`
/// followed by `text`. A target recorded before the model's prefixes existed (`prefix_rev` 0)
/// gets the raw text, so queries keep matching an index built without prefixes until it is
/// reindexed.
pub fn apply_prefix(spec: &EmbeddingModelSpec, target: &EmbeddingTarget, role: EmbedRole, text: &str) -> String {
    if target.prefix_rev == 0 || target.prefix_rev != spec.prefix_rev {
        return text.to_string();
    }
    format!("{}{}", spec.prefix(role), text)
}

/// Every embedding model AnyContext knows.
pub const EMBEDDING_CATALOG: &[EmbeddingModelSpec] = &[
    EmbeddingModelSpec {
        id: "multilingual-e5-small",
        name: "Multilingual E5 Small",
        runtime: EmbeddingRuntime::Local,
        license: "MIT",
        dimension: 384,
        max_tokens: 512,
        pooling: Pooling::Mean,
        query_prefix: "query: ",
        document_prefix: "passage: ",
        prefix_rev: 1,
    },
    EmbeddingModelSpec {
        id: "multilingual-e5-base",
        name: "Multilingual E5 Base",
        runtime: EmbeddingRuntime::Local,
        license: "MIT",
        dimension: 768,
        max_tokens: 512,
        pooling: Pooling::Mean,
        query_prefix: "query: ",
        document_prefix: "passage: ",
        prefix_rev: 1,
    },
    EmbeddingModelSpec {
        id: "bge-m3",
        name: "BGE-M3 (dense)",
        runtime: EmbeddingRuntime::Local,
        license: "MIT",
        dimension: 1024,
        max_tokens: 512,
        pooling: Pooling::Cls,
        query_prefix: "",
        document_prefix: "",
        prefix_rev: 0,
    },
    EmbeddingModelSpec {
        id: "text-embedding-3-small",
        name: "OpenAI text-embedding-3-small",
        runtime: EmbeddingRuntime::Cloud("openai"),
        license: "proprietary (API)",
        dimension: 1536,
        max_tokens: 8191,
        pooling: Pooling::Mean,
        query_prefix: "",
        document_prefix: "",
        prefix_rev: 0,
    },
    EmbeddingModelSpec {
        id: "gemini-embedding-2",
        name: "Google gemini-embedding-2",
        runtime: EmbeddingRuntime::Cloud("gemini"),
        license: "proprietary (API)",
        dimension: 1536,
        max_tokens: 2048,
        pooling: Pooling::Mean,
        // Google's embeddings guide: the model takes task prefixes instead of `task_type`.
        query_prefix: "task: search result | query: ",
        document_prefix: "title: none | text: ",
        prefix_rev: 1,
    },
    EmbeddingModelSpec {
        id: "nomic-embed-text",
        name: "nomic-embed-text (Ollama)",
        runtime: EmbeddingRuntime::Cloud("ollama"),
        license: "Apache-2.0",
        dimension: 768,
        max_tokens: 2048,
        pooling: Pooling::Mean,
        query_prefix: "search_query: ",
        document_prefix: "search_document: ",
        prefix_rev: 1,
    },
    EmbeddingModelSpec {
        id: MOCK_MODEL_ID,
        name: "Mock vectors (tests only)",
        runtime: EmbeddingRuntime::Mock,
        license: "-",
        dimension: 1536,
        max_tokens: 8192,
        pooling: Pooling::Mean,
        query_prefix: "",
        document_prefix: "",
        prefix_rev: 0,
    },
];

/// Id of the mock model.
pub const MOCK_MODEL_ID: &str = "mock-embed";

/// Finds a model by id, ignoring case; also accepts `mock` for the mock model.
pub fn find_embedding_model(id: &str) -> Option<&'static EmbeddingModelSpec> {
    let clean = id.trim();
    if clean.eq_ignore_ascii_case("mock") {
        return EMBEDDING_CATALOG.iter().find(|m| m.is_mock());
    }
    EMBEDDING_CATALOG.iter().find(|m| m.id.eq_ignore_ascii_case(clean))
}

/// The catalog entry of a recorded target (same provider and model), whatever its dimension
/// and prefix revision.
pub fn spec_for_target(target: &EmbeddingTarget) -> Option<&'static EmbeddingModelSpec> {
    EMBEDDING_CATALOG
        .iter()
        .find(|m| m.provider() == target.provider && m.id == target.model)
}

/// The models a user can pick (everything but the mock).
pub fn selectable_models() -> impl Iterator<Item = &'static EmbeddingModelSpec> {
    EMBEDDING_CATALOG.iter().filter(|m| !m.is_mock())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestion::OnnxModelManager;
    use crate::ingestion::OnnxModelCategory;

    #[test]
    fn catalog_is_consistent() {
        let mut ids = std::collections::HashSet::new();
        for m in EMBEDDING_CATALOG {
            assert!(ids.insert(m.id), "duplicate id {}", m.id);
            assert!(m.dimension > 0 && m.max_tokens > 0, "{}", m.id);
            assert_eq!(m.prefix_rev == 0, m.query_prefix.is_empty() && m.document_prefix.is_empty(), "{}", m.id);
            if m.is_local() {
                let files = OnnxModelManager::find_spec(m.id).unwrap_or_else(|| panic!("{} has no model files", m.id));
                assert_eq!(files.category, OnnxModelCategory::Embedding, "{}", m.id);
                assert!(files.is_available() && files.files.len() == 2, "{}", m.id);
            }
        }
    }

    #[test]
    fn lookup_and_targets() {
        assert_eq!(find_embedding_model("Multilingual-E5-Small").unwrap().dimension, 384);
        assert!(find_embedding_model("mock").unwrap().is_mock());
        assert!(find_embedding_model("unknown").is_none());
        let e5 = find_embedding_model("multilingual-e5-small").unwrap();
        assert_eq!(e5.target().key(), "local/multilingual-e5-small@384+p1");
        assert_eq!(spec_for_target(&e5.target()), Some(e5));
        assert!(selectable_models().all(|m| !m.is_mock()));
    }

    #[test]
    fn prefixes_follow_the_recorded_space() {
        let e5 = find_embedding_model("multilingual-e5-small").unwrap();
        assert_eq!(apply_prefix(e5, &e5.target(), EmbedRole::Query, "onde"), "query: onde");
        assert_eq!(apply_prefix(e5, &e5.target(), EmbedRole::Document, "texto"), "passage: texto");
        let m3 = find_embedding_model("bge-m3").unwrap();
        assert_eq!(apply_prefix(m3, &m3.target(), EmbedRole::Query, "onde"), "onde");
        // A Gemini index from before the prefixes keeps getting raw queries until it is reindexed.
        let gemini = find_embedding_model("gemini-embedding-2").unwrap();
        let legacy = EmbeddingTarget::parse("gemini/gemini-embedding-2").unwrap();
        assert_eq!(apply_prefix(gemini, &legacy, EmbedRole::Query, "q"), "q");
        assert_eq!(apply_prefix(gemini, &gemini.target(), EmbedRole::Query, "q"), "task: search result | query: q");
    }
}
