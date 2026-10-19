//! Native Hybrid RAG Pipeline and RFC-042 Batch Retrieval Engine in Rust.
//!
//! Unifies dense vector similarity (LanceDB), Okapi BM25 lexical ranking,
//! Reciprocal Rank Fusion (RRF), cross-query SHA-256 deduplication with score accumulation,
//! a per-source cap in score order, and density budgeting entirely in native Rust.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::storage::lancedb::{NativeLanceStore, ScoredVectorResult};
use crate::ingestion::enricher::NativeContextualEnricher;
use crate::ingestion::embedding::EmbeddingTarget;
use crate::retrieval::bm25::BM25Index;
use crate::retrieval::rrf::reciprocal_rank_fusion;
use crate::retrieval::diversifier::{RankedChunk, apply_source_cap, apply_density_budget};
use crate::retrieval::query::QueryPreprocessor;
use crate::retrieval::token_budget::estimate_token_count;
use actx_lm::traits::LmProvider;

/// Preset profiles for hybrid retrieval and RAG candidate boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetrievalPreset {
    Turbo,
    Balanced,
    Deep,
}

impl Default for RetrievalPreset {
    fn default() -> Self {
        Self::Balanced
    }
}

impl RetrievalPreset {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().trim() {
            "turbo" | "fast" => Self::Turbo,
            "deep" | "deep_research" | "research" => Self::Deep,
            _ => Self::Balanced,
        }
    }

    pub fn candidate_pool_k(&self) -> usize {
        match self {
            Self::Turbo => 50,
            Self::Balanced => 100,
            Self::Deep => 150,
        }
    }

    pub fn top_k(&self) -> usize {
        match self {
            Self::Turbo => 10,
            Self::Balanced => 20,
            Self::Deep => 40,
        }
    }

    pub fn min_score(&self) -> f64 {
        match self {
            Self::Turbo => 0.012,
            Self::Balanced => 0.008,
            Self::Deep => 0.005,
        }
    }

    pub fn max_chunks_per_source(&self) -> usize {
        match self {
            Self::Turbo => 2,
            Self::Balanced => 3,
            Self::Deep => 5,
        }
    }

    pub fn max_density_chars(&self) -> usize {
        match self {
            Self::Turbo => 20_000,
            Self::Balanced => 40_000,
            Self::Deep => 60_000,
        }
    }

    pub fn rrf_k(&self) -> usize {
        60
    }

    pub fn build_request(&self, query_text: impl Into<String>, workspace: Option<String>) -> HybridSearchRequest {
        HybridSearchRequest {
            sub_query_id: None,
            query_text: query_text.into(),
            query_vector: None,
            workspace,
            target_workspaces: Vec::new(),
            linked_sources: Vec::new(),
            top_k: self.top_k(),
            candidate_pool_k: self.candidate_pool_k(),
            min_score: self.min_score(),
            max_chunks_per_source: self.max_chunks_per_source(),
            max_density_chars: self.max_density_chars(),
            table_name: "workspace_chunks".to_string(),
            rrf_k: self.rrf_k(),
            rerank_mode: None,
        }
    }
}

/// Request payload for native hybrid retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchRequest {
    pub sub_query_id: Option<String>,
    pub query_text: String,
    pub query_vector: Option<Vec<f32>>,
    pub workspace: Option<String>,
    pub target_workspaces: Vec<String>,
    pub linked_sources: Vec<String>,
    pub top_k: usize,
    pub candidate_pool_k: usize,
    pub min_score: f64,
    pub max_chunks_per_source: usize,
    pub max_density_chars: usize,
    pub table_name: String,
    pub rrf_k: usize,
    #[serde(default)]
    pub rerank_mode: Option<String>,
}

impl HybridSearchRequest {
    pub fn from_preset(preset: RetrievalPreset, query_text: impl Into<String>, workspace: Option<String>) -> Self {
        preset.build_request(query_text, workspace)
    }
}

impl Default for HybridSearchRequest {
    fn default() -> Self {
        Self {
            sub_query_id: None,
            query_text: String::new(),
            query_vector: None,
            workspace: None,
            target_workspaces: Vec::new(),
            linked_sources: Vec::new(),
            top_k: 20,
            candidate_pool_k: 100,
            min_score: 0.0,
            max_chunks_per_source: 3,
            max_density_chars: 40_000,
            table_name: "workspace_chunks".to_string(),
            rrf_k: 60,
            rerank_mode: None,
        }
    }
}

/// Scored, diversified, and deduplicated chunk result with complete provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridSearchResult {
    pub chunk_id: String,
    pub file_name: String,
    pub file_path: String,
    pub workspace: String,
    pub text: String,
    pub content_type: String,
    pub content_hash: String,
    pub score: f64,
    pub dense_score: Option<f32>,
    pub sparse_score: Option<f32>,
    pub token_count: usize,
    pub matched_subqueries: Vec<String>,
    pub last_modified: Option<String>,
    /// Section or symbol path of the chunk, when stored (ADR-0142).
    #[serde(default)]
    pub header_path: Option<String>,
    /// File line range, only for chunkers that emit file line numbers.
    #[serde(default)]
    pub start_line: Option<u32>,
    #[serde(default)]
    pub end_line: Option<u32>,
}

/// Query embedding model used until `set_embedding` is called.
pub const DEFAULT_QUERY_EMBEDDING_MODEL: &str = "text-embedding-3-small";

/// Maximum candidates re-scored by the cross-encoder per query (ADR-130).
pub const MAX_RERANK_CANDIDATES: usize = 30;

/// RRF floor for `min_score`: 1/(60 + 140), so every candidate of a pool of up to 140 per list
/// passes. It guards against empty matches and is not a relevance cutoff (ADR-0139).
pub const RRF_MIN_SCORE: f64 = 0.005;

/// Workspace a request searches: its own, or `Default` when it names none. `Default` is filtered
/// like any other workspace, so it never returns other workspaces' chunks (BUG-023).
fn primary_workspace(req: &HybridSearchRequest) -> &str {
    req.workspace.as_deref().filter(|w| !w.is_empty()).unwrap_or("Default")
}

fn parse_last_modified(value: &Option<String>) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    value.as_deref().and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
}

/// Relevance order: score descending; on an exact tie, newer `last_modified` first (missing or
/// invalid dates last). Recency never changes a score, so it cannot lift a chunk past a better one.
pub(crate) fn compare_by_relevance(a: &HybridSearchResult, b: &HybridSearchResult) -> std::cmp::Ordering {
    b.score
        .partial_cmp(&a.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| parse_last_modified(&b.last_modified).cmp(&parse_last_modified(&a.last_modified)))
}

/// How the dense half of a hybrid search went for a query (IMP-003). A query without dense
/// search still answers from BM25 (ADR-110); this only makes that visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryEmbeddingOutcome {
    /// The query was embedded (or a vector was supplied) and dense search ran.
    Dense,
    /// No embedding provider was configured for the pipeline.
    NoProvider,
    /// The query target is the mock one (no embedding API key): its vectors are all the same, so
    /// dense search is skipped (BUG-019).
    NoEmbeddingKey,
    /// The embedding call failed or returned no vector.
    Failed(String),
    /// The query was embedded but the vector search itself failed (BUG-021).
    SearchFailed(String),
}

impl QueryEmbeddingOutcome {
    /// One-line description for status surfaces.
    pub fn describe(&self) -> String {
        match self {
            QueryEmbeddingOutcome::Dense => "dense + BM25 (last query)".to_string(),
            QueryEmbeddingOutcome::NoProvider => "BM25 only (last query): no embedding provider".to_string(),
            QueryEmbeddingOutcome::NoEmbeddingKey => "BM25 only (last query): no embedding API key (/keys)".to_string(),
            QueryEmbeddingOutcome::Failed(reason) => format!("BM25 only (last query): query embedding failed: {reason}"),
            QueryEmbeddingOutcome::SearchFailed(reason) => format!("BM25 only (last query): dense search failed: {reason}"),
        }
    }
}

static LAST_QUERY_EMBEDDING: std::sync::Mutex<Option<QueryEmbeddingOutcome>> = std::sync::Mutex::new(None);

/// Outcome of the query embedding of the most recent hybrid search in this process, if any.
pub fn last_query_embedding_outcome() -> Option<QueryEmbeddingOutcome> {
    LAST_QUERY_EMBEDDING.lock().ok().and_then(|o| o.clone())
}

/// Status line for the dense half of retrieval, from the most recent search in this process.
pub fn dense_search_status() -> String {
    last_query_embedding_outcome()
        .map(|o| o.describe())
        .unwrap_or_else(|| "n/a (no query yet in this session)".to_string())
}

static LAST_RERANK_LATENCY_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

/// Wall-clock latency of the most recent neural re-ranking pass in this process, if any.
pub fn last_rerank_latency_ms() -> Option<u64> {
    match LAST_RERANK_LATENCY_MS.load(std::sync::atomic::Ordering::Relaxed) {
        u64::MAX => None,
        v => Some(v),
    }
}

/// Scores `texts` with the cross-encoder on a blocking thread so the async executor
/// (and the TUI) never stall on CPU inference. `None` means "keep RRF ordering".
async fn neural_rerank_scores(
    reranker: &Arc<actx_onnx::OnnxReranker>,
    query: &str,
    texts: Vec<String>,
) -> Option<Vec<f32>> {
    if texts.is_empty() || !reranker.is_neural_active() {
        return None;
    }
    let expected = texts.len();
    let reranker = Arc::clone(reranker);
    let query = query.to_string();
    let started = std::time::Instant::now();
    let scores = tokio::task::spawn_blocking(move || reranker.score_batch(&query, &texts))
        .await
        .ok()?;
    if scores.len() != expected {
        return None;
    }
    LAST_RERANK_LATENCY_MS.store(started.elapsed().as_millis() as u64, std::sync::atomic::Ordering::Relaxed);
    Some(scores)
}

/// Applies Stage-2 re-ranking to the best `MAX_RERANK_CANDIDATES` unprotected items (by current
/// score). Re-scored items receive calibrated `[0,1]` scores; the remaining tail is rescaled to
/// stay strictly below every re-scored item while keeping its relative RRF order. `protected` ids
/// keep their original score untouched and do not consume cross-encoder slots.
async fn apply_neural_rerank(
    reranker: &Arc<actx_onnx::OnnxReranker>,
    query: &str,
    items: &mut [HybridSearchResult],
    protected: &std::collections::HashSet<String>,
) {
    if items.is_empty() {
        return;
    }
    items.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let head: Vec<usize> = (0..items.len())
        .filter(|&i| !protected.contains(&items[i].chunk_id))
        .take(MAX_RERANK_CANDIDATES)
        .collect();
    let texts: Vec<String> = head.iter().map(|&i| items[i].text.clone()).collect();
    let Some(scores) = neural_rerank_scores(reranker, query, texts).await else {
        return;
    };

    let mut rescored = vec![false; items.len()];
    let mut min_head = f64::MAX;
    for (&i, &s) in head.iter().zip(&scores) {
        items[i].score = s as f64;
        rescored[i] = true;
        min_head = min_head.min(s as f64);
    }
    let is_tail = |i: usize, c: &HybridSearchResult| !rescored[i] && !protected.contains(&c.chunk_id);
    let tail_max = items
        .iter()
        .enumerate()
        .filter(|(i, c)| is_tail(*i, c))
        .map(|(_, c)| c.score)
        .fold(0.0_f64, f64::max);
    if tail_max > 0.0 {
        let ceiling = min_head * 0.5;
        for i in 0..items.len() {
            if is_tail(i, &items[i]) {
                items[i].score = ceiling * (items[i].score / tail_max).max(0.0);
            }
        }
    }
}

/// Section path and file line range of a stored chunk.
#[derive(Debug, Clone, Default)]
struct Provenance {
    header_path: Option<String>,
    start_line: Option<u32>,
    end_line: Option<u32>,
}

impl Provenance {
    fn of(row: &ScoredVectorResult) -> Self {
        Self {
            header_path: row.header_path.clone(),
            start_line: row.start_line,
            end_line: row.end_line,
        }
    }
}

/// Unified Native Hybrid Retrieval and Reranking Pipeline.
pub struct NativeHybridPipeline {
    db_path: PathBuf,
    lance_store: Arc<NativeLanceStore>,
    bm25_indices: Arc<RwLock<HashMap<String, BM25Index>>>,
    lm_client: Option<Arc<dyn LmProvider>>,
    embedding_model: String,
    reranker: Option<Arc<actx_onnx::OnnxReranker>>,
    rerank_mode: String,
    last_query_embedding: std::sync::Mutex<Option<QueryEmbeddingOutcome>>,
    /// Outcome recorded when there is no embedding provider.
    missing_provider_outcome: QueryEmbeddingOutcome,
    /// Whether `Global` rows take part in dense search (their vectors share the query's space).
    global_dense: bool,
}

impl NativeHybridPipeline {
    /// Opens or connects to the underlying storage at `db_path`.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let p = db_path.as_ref().to_path_buf();
        let lance_store = Arc::new(NativeLanceStore::open(&p)?);
        Ok(Self {
            db_path: p,
            lance_store,
            bm25_indices: Arc::new(RwLock::new(HashMap::new())),
            lm_client: None,
            embedding_model: DEFAULT_QUERY_EMBEDDING_MODEL.to_string(),
            reranker: None,
            rerank_mode: "auto".to_string(),
            last_query_embedding: std::sync::Mutex::new(None),
            missing_provider_outcome: QueryEmbeddingOutcome::NoProvider,
            global_dense: false,
        })
    }

    /// Creates pipeline with an existing [`NativeLanceStore`] and optional [`LmProvider`].
    pub fn new(lance_store: Arc<NativeLanceStore>, lm_client: Option<Arc<dyn LmProvider>>) -> Self {
        let p = lance_store.db_path().to_path_buf();
        Self {
            db_path: p,
            lance_store,
            bm25_indices: Arc::new(RwLock::new(HashMap::new())),
            lm_client,
            embedding_model: DEFAULT_QUERY_EMBEDDING_MODEL.to_string(),
            reranker: None,
            rerank_mode: "auto".to_string(),
            last_query_embedding: std::sync::Mutex::new(None),
            missing_provider_outcome: QueryEmbeddingOutcome::NoProvider,
            global_dense: false,
        }
    }

    /// Outcome of the query embedding of this pipeline's most recent search, if any.
    pub fn last_query_embedding(&self) -> Option<QueryEmbeddingOutcome> {
        self.last_query_embedding.lock().ok().and_then(|o| o.clone())
    }

    fn record_query_embedding(&self, outcome: QueryEmbeddingOutcome) {
        if let Ok(mut last) = LAST_QUERY_EMBEDDING.lock() {
            *last = Some(outcome.clone());
        }
        if let Ok(mut last) = self.last_query_embedding.lock() {
            *last = Some(outcome);
        }
    }

    /// Sets the optional language model client for native on-the-fly embeddings.
    pub fn set_lm_client(&mut self, lm_client: Option<Arc<dyn LmProvider>>) {
        self.lm_client = lm_client;
    }

    /// Sets the embedding provider and model used for query vectors. They must match the ones
    /// used at ingestion time (see `ingestion::resolve_embedding_provider`), otherwise query and
    /// chunk vectors live in different spaces.
    pub fn set_embedding(&mut self, lm_client: Option<Arc<dyn LmProvider>>, model: impl Into<String>) {
        self.lm_client = lm_client;
        self.embedding_model = model.into();
    }

    /// Sets the query embedding from `ingestion::resolve_query_embedding_provider`. On the mock
    /// target the resolver gives no provider and the query is recorded as `NoEmbeddingKey`.
    pub fn set_query_embedding(&mut self, lm_client: Option<Arc<dyn LmProvider>>, target: &EmbeddingTarget) {
        self.set_embedding(lm_client, target.model.clone());
        self.missing_provider_outcome = if target.is_mock() {
            QueryEmbeddingOutcome::NoEmbeddingKey
        } else {
            QueryEmbeddingOutcome::NoProvider
        };
    }

    /// Lets `Global` rows take part in dense search. Off by default: they are placeholders
    /// until embedded with the query's target (`ingestion::global_dense_enabled`, BUG-019).
    pub fn set_global_dense(&mut self, enabled: bool) {
        self.global_dense = enabled;
    }

    /// Embedding model used for query vectors.
    pub fn embedding_model(&self) -> &str {
        &self.embedding_model
    }

    /// Sets the optional cross-encoder neural reranker.
    pub fn set_reranker(&mut self, reranker: Option<Arc<actx_onnx::OnnxReranker>>) {
        self.reranker = reranker;
    }

    /// Returns a reference to the active reranker, if present.
    pub fn reranker(&self) -> Option<&Arc<actx_onnx::OnnxReranker>> {
        self.reranker.as_ref()
    }

    /// Sets the global reranking mode ("auto", "on", "off").
    pub fn set_rerank_mode(&mut self, mode: &str) {
        self.rerank_mode = mode.to_string();
    }

    /// Returns the global reranking mode.
    pub fn rerank_mode(&self) -> &str {
        &self.rerank_mode
    }

    /// Returns the database root directory path.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Retrieves or loads the BM25 index for the given table.
    pub fn get_or_load_bm25(&self, table_name: &str) -> BM25Index {
        {
            let reader = self.bm25_indices.read().unwrap();
            if let Some(idx) = reader.get(table_name) {
                return idx.clone();
            }
        }

        let prefix = if table_name == "workspace_chunks" {
            "bm25_index.bin".to_string()
        } else {
            format!("bm25_{table_name}.bin")
        };
        let file_path = self.db_path.join(prefix);
        let index = if file_path.exists() {
            match BM25Index::load_from_file(file_path.to_str().unwrap_or("")) {
                Ok(idx) => idx,
                Err(err) => {
                    let log_dir = crate::storage::get_default_logs_dir();
                    let _ = std::fs::create_dir_all(&log_dir);
                    let log_file = log_dir.join("bm25_quarantine.log");
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_file) {
                        use std::io::Write;
                        let _ = writeln!(f, "[{}] Corrupted BM25 at '{:?}': {}", chrono::Utc::now().to_rfc3339(), file_path, err);
                    }
                    let corrupt_name = format!("{}.corrupt.{}", file_path.display(), chrono::Utc::now().timestamp());
                    let _ = std::fs::rename(&file_path, &corrupt_name);
                    BM25Index::new(None, None)
                }
            }
        } else {
            BM25Index::new(None, None)
        };

        let mut writer = self.bm25_indices.write().unwrap();
        writer.insert(table_name.to_string(), index.clone());
        index
    }

    /// Adds a chunk into the in-memory BM25 index for the specified table.
    pub fn add_chunk_to_bm25(
        &self,
        table_name: &str,
        id: String,
        text: String,
        file_name: String,
        file_path: String,
        workspace: String,
        content_type: String,
        last_modified: Option<String>,
    ) {
        // Ensure index is loaded
        let _ = self.get_or_load_bm25(table_name);
        let mut writer = self.bm25_indices.write().unwrap();
        if let Some(idx) = writer.get_mut(table_name) {
            idx.add_chunk(id, text, file_name, file_path, workspace, content_type, last_modified);
        }
    }

    /// Saves the BM25 index for the specified table to disk.
    pub fn save_bm25(&self, table_name: &str) -> Result<(), String> {
        let reader = self.bm25_indices.read().unwrap();
        if let Some(idx) = reader.get(table_name) {
            let prefix = if table_name == "workspace_chunks" {
                "bm25_index.bin".to_string()
            } else {
                format!("bm25_{table_name}.bin")
            };
            let file_path = self.db_path.join(prefix);
            if let Some(parent) = file_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            idx.save_to_file(file_path.to_str().unwrap_or(""))
                .map_err(|e| format!("Failed to save BM25 index: {e}"))?;
        }
        Ok(())
    }

    /// Loads the BM25 index from a specific file path.
    pub fn load_bm25_from_file(&self, table_name: &str, file_path: &str) -> Result<(), String> {
        let loaded = BM25Index::load_from_file(file_path)
            .map_err(|e| format!("Failed to load BM25 index from {file_path}: {e}"))?;
        let mut writer = self.bm25_indices.write().unwrap();
        writer.insert(table_name.to_string(), loaded);
        Ok(())
    }

    /// Executes single-query hybrid search asynchronously.
    pub async fn search_single_async(
        &self,
        req: &HybridSearchRequest,
    ) -> Result<Vec<HybridSearchResult>, String> {
        // 1. Build distinct workspace query targets
        let mut targets = vec![(primary_workspace(&req).to_string(), req.candidate_pool_k)];
        for tw in &req.target_workspaces {
            if !tw.is_empty() && !targets.iter().any(|(w, _)| w == tw) {
                targets.push((tw.clone(), std::cmp::max(10, req.candidate_pool_k / 2)));
            }
        }

        let mut raw_candidates_map: HashMap<String, ScoredVectorResult> = HashMap::new();
        let mut guaranteed_file_cids: Vec<String> = Vec::new();

        // 2. Query Preprocessing in Rust (AST/lexical pattern extraction)
        let processed = QueryPreprocessor::process(&req.query_text);
        let filenames = processed.filename_mentions;
        let temporal_clauses = processed.temporal_clauses;
        let expanded_query = processed.expanded_query;

        // 3. Metadata Filtering for Exact Filename Mentions
        if !filenames.is_empty() {
            for fn_name in &filenames {
                let fn_clean = fn_name.replace('\'', "''");
                let fn_clause =
                    format!("(file_name LIKE '%{fn_clean}%' OR file_path LIKE '%{fn_clean}%')");
                let combined_where = if !temporal_clauses.is_empty() {
                    format!("({fn_clause}) AND ({})", temporal_clauses.join(" OR "))
                } else {
                    fn_clause.clone()
                };

                for (ws_name, limit) in &targets {
                    let ws_filter = Some(ws_name.as_str());
                    let mut matches = self
                        .lance_store
                        .search_metadata_async(&combined_where, *limit, ws_filter, Some(&req.table_name))
                        .await
                        .unwrap_or_default();

                    if matches.is_empty() && !temporal_clauses.is_empty() {
                        matches = self
                            .lance_store
                            .search_metadata_async(&fn_clause, *limit, ws_filter, Some(&req.table_name))
                            .await
                            .unwrap_or_default();
                    }

                    for mut sc in matches {
                        sc.score = 1.0; // Max score boost for explicitly requested file chunks
                        let cid = if !sc.id.is_empty() {
                            sc.id.clone()
                        } else {
                            format!(
                                "{}::{}",
                                sc.file_path,
                                &sc.text.chars().take(80).collect::<String>()
                            )
                        };
                        if !guaranteed_file_cids.contains(&cid) {
                            guaranteed_file_cids.push(cid.clone());
                        }
                        raw_candidates_map.insert(cid, sc);
                    }
                }
            }
        }

        // 4. Temporal Metadata Filtering
        if !temporal_clauses.is_empty() {
            let where_or = temporal_clauses.join(" OR ");
            for (ws_name, limit) in &targets {
                let ws_filter = Some(ws_name.as_str());
                if let Ok(matches) = self
                    .lance_store
                    .search_metadata_async(&where_or, *limit, ws_filter, Some(&req.table_name))
                    .await
                {
                    for mut sc in matches {
                        let cid = if !sc.id.is_empty() {
                            sc.id.clone()
                        } else {
                            format!(
                                "{}::{}",
                                sc.file_path,
                                &sc.text.chars().take(80).collect::<String>()
                            )
                        };
                        if let Some(existing) = raw_candidates_map.get_mut(&cid) {
                            existing.score = existing.score.max(0.95);
                        } else {
                            sc.score = 0.95;
                            raw_candidates_map.insert(cid, sc);
                        }
                    }
                }
            }
        }

        // 5. Query Vector Resolution (Passed in or computed via actx-lm)
        // A failed or missing embedding leaves BM25 only (ADR-110); the outcome is recorded (IMP-003).
        let (query_vec, outcome) = if let Some(ref v) = req.query_vector {
            (Some(v.clone()), QueryEmbeddingOutcome::Dense)
        } else if let Some(ref lm) = self.lm_client {
            match lm.embed(&self.embedding_model, &[req.query_text.clone()]).await {
                Ok(mut vecs) if !vecs.is_empty() => (Some(vecs.swap_remove(0)), QueryEmbeddingOutcome::Dense),
                Ok(_) => (None, QueryEmbeddingOutcome::Failed("empty embedding answer".to_string())),
                Err(e) => (None, QueryEmbeddingOutcome::Failed(e.to_string())),
            }
        } else {
            (None, self.missing_provider_outcome.clone())
        };
        self.record_query_embedding(outcome);

        // 6. Dense Vector Similarity Search. `Global` rows take part only when their vectors
        // share the query's space; otherwise BM25 and metadata matches still find them (BUG-019).
        let dense_filter = (!self.global_dense).then_some("workspace <> 'Global'");
        if let Some(q_vec) = query_vec {
            let mut search_error = None;
            for (ws_name, limit) in &targets {
                if !self.global_dense && ws_name == "Global" {
                    continue;
                }
                let ws_filter = Some(ws_name.as_str());
                // A failed search is recorded, never dropped silently (BUG-021).
                let matches = match self
                    .lance_store
                    .search_vector_async(q_vec.clone(), *limit, ws_filter, dense_filter, Some(&req.table_name))
                    .await
                {
                    Ok(matches) => matches,
                    Err(e) => {
                        search_error.get_or_insert(e);
                        continue;
                    }
                };
                for sc in matches {
                    let cid = if !sc.id.is_empty() {
                        sc.id.clone()
                    } else {
                        format!(
                            "{}::{}",
                            sc.file_path,
                            &sc.text.chars().take(80).collect::<String>()
                        )
                    };
                    if let Some(existing) = raw_candidates_map.get_mut(&cid) {
                        existing.score = existing.score.max(sc.score);
                    } else {
                        raw_candidates_map.insert(cid, sc);
                    }
                }
            }
            if let Some(e) = search_error {
                self.record_query_embedding(QueryEmbeddingOutcome::SearchFailed(e));
            }
        }

        // 7. Order dense candidates
        let mut dense_candidates: Vec<ScoredVectorResult> =
            raw_candidates_map.into_values().collect();
        dense_candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let dense_ids: Vec<String> = dense_candidates.iter().map(|c| c.id.clone()).collect();
        let mut dense_map: HashMap<String, ScoredVectorResult> =
            dense_candidates.into_iter().map(|c| (c.id.clone(), c)).collect();

        // 8. Sparse Lexical BM25 Search
        let bm25_index = self.get_or_load_bm25(&req.table_name);
        // `Default` is a workspace like any other: it sees its own files and `Global` (BUG-023).
        let target_ws = Some(primary_workspace(&req));
        let sparse_matches = bm25_index.search(&expanded_query, req.candidate_pool_k, target_ws);
        let sparse_ids: Vec<String> = sparse_matches.iter().map(|(id, _)| id.clone()).collect();
        let sparse_score_map: HashMap<String, f32> = sparse_matches.into_iter().collect();

        // 9. Reciprocal Rank Fusion (RRF)
        let fused_results = reciprocal_rank_fusion(&dense_ids, &sparse_ids, Some(req.rrf_k));

        let sub_query_tag = req
            .sub_query_id
            .clone()
            .unwrap_or_else(|| req.query_text.clone());

        // 10. Stored rows of BM25-only hits: the BM25 file holds the enveloped text it indexes and
        // no provenance, so text, content hash and provenance are read from LanceDB in one lookup
        // (ADR-0142, ADR-0143).
        let bm25_only_ids: Vec<String> = fused_results
            .iter()
            .filter(|r| !dense_map.contains_key(&r.id))
            .map(|r| r.id.clone())
            .collect();
        let mut stored_rows = self
            .lookup_stored_rows(&bm25_only_ids, Some(primary_workspace(&req)), &req.table_name)
            .await;

        // 11. Assemble HybridSearchResult candidates
        let mut results = Vec::with_capacity(fused_results.len());
        for res in fused_results {
            let chunk_id = res.id;
            let dense_entry = dense_map.remove(&chunk_id);
            let sparse_score = sparse_score_map.get(&chunk_id).copied();

            let (file_name, file_path, workspace, text, content_type, content_hash, dense_score, last_modified, origin) =
                if let Some(ref d) = dense_entry {
                    (
                        d.file_name.clone(),
                        d.file_path.clone(),
                        d.workspace.clone(),
                        d.text.clone(),
                        d.content_type.clone().unwrap_or_else(|| "Local Document".to_string()),
                        d.content_hash.clone().unwrap_or_default(),
                        Some(d.score),
                        d.last_modified.clone(),
                        Provenance::of(d),
                    )
                } else if let Some(doc) = bm25_index.get_doc_by_id(&chunk_id) {
                    let row = stored_rows.remove(&(chunk_id.clone(), doc.file_path.replace('\\', "/")));
                    let (text, content_hash, origin) = match row {
                        Some(r) => {
                            let origin = Provenance::of(&r);
                            (r.text, r.content_hash.unwrap_or_default(), origin)
                        }
                        // No row (index out of step or lookup error): drop the envelope at least.
                        None => (
                            NativeContextualEnricher::strip_envelope(&doc.text).to_string(),
                            String::new(),
                            Provenance::default(),
                        ),
                    };
                    (
                        doc.file_name.clone(),
                        doc.file_path.clone(),
                        doc.workspace.clone(),
                        text,
                        doc.content_type.clone(),
                        content_hash,
                        None,
                        doc.last_modified.clone(),
                        origin,
                    )
                } else {
                    continue;
                };

            let clear_text = if text.starts_with("enc::") {
                let sec = crate::security::NativeSecurityEngine::get_instance();
                sec.decrypt_text(&text)
            } else {
                text
            };

            let token_count = estimate_token_count(&clear_text);
            let is_guaranteed = guaranteed_file_cids.contains(&chunk_id);

            let final_score = if is_guaranteed { 1.0 } else { res.score };

            results.push(HybridSearchResult {
                chunk_id,
                file_name,
                file_path,
                workspace,
                text: clear_text,
                content_type,
                content_hash,
                score: final_score,
                dense_score,
                sparse_score,
                token_count,
                matched_subqueries: vec![sub_query_tag.clone()],
                last_modified,
                header_path: origin.header_path,
                start_line: origin.start_line,
                end_line: origin.end_line,
            });
        }

        results.sort_by(compare_by_relevance);

        // Filter by min_score if set
        if req.min_score > 0.0 {
            results.retain(|c| c.score >= req.min_score);
        }

        // Stage 2: Neural Cross-Encoder Re-Ranking (Frente B)
        let should_rerank = self.rerank_mode != "off" && req.rerank_mode.as_deref() != Some("off");
        if should_rerank {
            if let Some(ref reranker) = self.reranker {
                let protected: std::collections::HashSet<String> = guaranteed_file_cids.iter().cloned().collect();
                apply_neural_rerank(reranker, &req.query_text, &mut results, &protected).await;
            }
        }

        // Sort descending by score (neural cross-encoder or RRF fallback)
        results.sort_by(compare_by_relevance);

        // Per-source cap in score order
        let ranked_chunks: Vec<RankedChunk> = results
            .iter()
            .map(|c| RankedChunk {
                id: c.chunk_id.clone(),
                score: c.score,
                file_path: c.file_path.clone(),
                file_name: c.file_name.clone(),
                text: c.text.clone(),
                workspace: c.workspace.clone(),
                content_type: c.content_type.clone(),
            })
            .collect();

        let capped_ranked = apply_source_cap(ranked_chunks, req.max_chunks_per_source, req.top_k);

        // Density budgeting
        let budgeted_ranked = apply_density_budget(capped_ranked, req.max_density_chars);

        let mut candidate_map: HashMap<String, HybridSearchResult> =
            results.into_iter().map(|c| (c.chunk_id.clone(), c)).collect();

        let mut final_results = Vec::with_capacity(budgeted_ranked.len());
        for b in budgeted_ranked {
            if let Some(res) = candidate_map.remove(&b.id) {
                final_results.push(res);
            }
        }

        Ok(final_results)
    }

    /// Reads the stored rows of `ids` from LanceDB, keyed by chunk id and normalized file path
    /// (chunk ids repeat across files with the same content). A lookup error leaves the hits
    /// without a row: the answer still works, with the BM25 text and a less precise citation.
    async fn lookup_stored_rows(
        &self,
        ids: &[String],
        workspace: Option<&str>,
        table_name: &str,
    ) -> HashMap<(String, String), ScoredVectorResult> {
        if ids.is_empty() {
            return HashMap::new();
        }
        let list = ids
            .iter()
            .map(|id| format!("'{}'", id.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        let ws_filter = workspace.filter(|w| !w.is_empty());
        // The same id can exist in several files and in the Global workspace.
        let limit = ids.len() * 4;
        self.lance_store
            .search_metadata_async(&format!("id IN ({list})"), limit, ws_filter, Some(table_name))
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|row| ((row.id.clone(), row.file_path.replace('\\', "/")), row))
            .collect()
    }

    /// Single-query hybrid retrieval synchronous wrapper.
    pub fn search(&self, req: HybridSearchRequest) -> Result<Vec<HybridSearchResult>, String> {
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(self.search_single_async(&req))
            })
        } else if let Some(rt) = self.lance_store.runtime() {
            rt.block_on(self.search_single_async(&req))
        } else {
            let temp_rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            temp_rt.block_on(self.search_single_async(&req))
        }
    }

    /// RFC-042: Concurrent multi-query batch retrieval with cross-query chunk deduplication,
    /// RRF score accumulation, a per-source cap in score order, and density budgeting.
    pub async fn retrieve_hybrid_batch_async(
        &self,
        requests: &[HybridSearchRequest],
    ) -> Result<Vec<HybridSearchResult>, String> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        // 1. Concurrently execute all sub-queries in Tokio
        let futures: Vec<_> = requests
            .iter()
            .map(|req| self.search_single_async(req))
            .collect();

        let all_results = futures::future::join_all(futures).await;

        // 2. Cross-query deduplication and score accumulation
        let mut dedup_map: HashMap<String, HybridSearchResult> = HashMap::new();

        for (req_idx, sub_res) in all_results.into_iter().enumerate() {
            let chunks = sub_res?;
            let sub_req = &requests[req_idx];
            let sub_label = sub_req
                .sub_query_id
                .clone()
                .unwrap_or_else(|| sub_req.query_text.clone());

            for chunk in chunks {
                // Keyed by chunk id: `content_hash` is the hash of the whole source file, shared
                // by all its chunks (ADR-0143). Files with identical content share chunk ids.
                let key = if !chunk.chunk_id.is_empty() {
                    chunk.chunk_id.clone()
                } else {
                    format!("{}::{}", chunk.file_path, &chunk.text)
                };

                if let Some(existing) = dedup_map.get_mut(&key) {
                    // Accumulate per-sub-query scores (RRF or cross-encoder; RFC-042 multi-query boost)
                    existing.score += chunk.score;
                    if !existing.matched_subqueries.contains(&sub_label) {
                        existing.matched_subqueries.push(sub_label.clone());
                    }
                    if let Some(d) = chunk.dense_score {
                        existing.dense_score =
                            Some(existing.dense_score.map_or(d, |curr| curr.max(d)));
                    }
                    if let Some(s) = chunk.sparse_score {
                        existing.sparse_score =
                            Some(existing.sparse_score.map_or(s, |curr| curr.max(s)));
                    }
                } else {
                    dedup_map.insert(key, chunk);
                }
            }
        }

        let first_req = &requests[0];
        let mut candidates: Vec<HybridSearchResult> = dedup_map.into_values().collect();

        // Each sub-query already applied `min_score` (on RRF scores) and Stage-2 re-ranking
        // against its own text, protecting guaranteed filename matches. Accumulated scores are
        // therefore not re-filtered or re-scored here: doing so would compare RRF thresholds
        // against cross-encoder scores, overwrite pinned matches and repeat CPU inference.

        // Sort descending by accumulated score (neural or RRF fallback)
        candidates.sort_by(compare_by_relevance);

        // 4. Per-source cap in score order
        let ranked_chunks: Vec<RankedChunk> = candidates
            .iter()
            .map(|c| RankedChunk {
                id: c.chunk_id.clone(),
                score: c.score,
                file_path: c.file_path.clone(),
                file_name: c.file_name.clone(),
                text: c.text.clone(),
                workspace: c.workspace.clone(),
                content_type: c.content_type.clone(),
            })
            .collect();

        let capped_ranked =
            apply_source_cap(ranked_chunks, first_req.max_chunks_per_source, first_req.top_k);

        // 5. Density Budgeting
        let budgeted_ranked = apply_density_budget(capped_ranked, first_req.max_density_chars);

        // 6. Map back to full HybridSearchResult preserving metadata and matched subqueries
        let mut candidate_map: HashMap<String, HybridSearchResult> =
            candidates.into_iter().map(|c| (c.chunk_id.clone(), c)).collect();

        let mut final_results = Vec::with_capacity(budgeted_ranked.len());
        for rc in budgeted_ranked {
            if let Some(mut full_chunk) = candidate_map.remove(&rc.id) {
                // Ensure truncated text / scores reflect final budgeted slice
                full_chunk.text = rc.text;
                full_chunk.score = rc.score;
                full_chunk.token_count = estimate_token_count(&full_chunk.text);
                final_results.push(full_chunk);
            }
        }

        Ok(final_results)
    }

    /// Synchronous wrapper for batch retrieval.
    pub fn retrieve_hybrid_batch(
        &self,
        requests: Vec<HybridSearchRequest>,
    ) -> Result<Vec<HybridSearchResult>, String> {
        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(self.retrieve_hybrid_batch_async(&requests))
            })
        } else if let Some(rt) = self.lance_store.runtime() {
            rt.block_on(self.retrieve_hybrid_batch_async(&requests))
        } else {
            let temp_rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            temp_rt.block_on(self.retrieve_hybrid_batch_async(&requests))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_pipeline_creation() {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_pipeline_{}", std::process::id()));
        let pipeline = NativeHybridPipeline::open(&temp_dir);
        assert!(pipeline.is_ok());

        let p = pipeline.unwrap();
        p.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-1".to_string(),
            "Rust async Tokio pipeline with Okapi BM25 and LanceDB".to_string(),
            "pipeline.rs".to_string(),
            "/src/retrieval/pipeline.rs".to_string(),
            "Default".to_string(),
            "Local Document".to_string(),
            Some("2026-10-10T12:00:00Z".to_string()),
        );

        let bm25 = p.get_or_load_bm25("workspace_chunks");
        assert_eq!(bm25.len(), 1);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_batch_retrieval_cross_dedup() {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_batch_{}", std::process::id()));
        let pipeline = NativeHybridPipeline::open(&temp_dir).unwrap();

        pipeline.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-a".to_string(),
            "authentication token JWT validation in Rust".to_string(),
            "auth.rs".to_string(),
            "/src/auth.rs".to_string(),
            "Default".to_string(),
            "Local Document".to_string(),
            Some("2026-10-10T12:00:00Z".to_string()),
        );

        pipeline.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-b".to_string(),
            "database connection pool with SQL query execution".to_string(),
            "db.rs".to_string(),
            "/src/db.rs".to_string(),
            "Default".to_string(),
            "Local Document".to_string(),
            Some("2026-10-09T10:00:00Z".to_string()),
        );

        let req1 = HybridSearchRequest {
            sub_query_id: Some("sub-1".to_string()),
            query_text: "authentication token JWT".to_string(),
            ..Default::default()
        };

        let req2 = HybridSearchRequest {
            sub_query_id: Some("sub-2".to_string()),
            query_text: "token validation".to_string(),
            ..Default::default()
        };

        let batch_results = pipeline
            .retrieve_hybrid_batch(vec![req1, req2])
            .unwrap();

        assert!(!batch_results.is_empty());
        let top_match = &batch_results[0];
        assert_eq!(top_match.chunk_id, "chunk-a");
        // chunk-a matched BOTH sub-queries, so matched_subqueries must contain both!
        assert!(top_match.matched_subqueries.contains(&"sub-1".to_string()));
        assert!(top_match.matched_subqueries.contains(&"sub-2".to_string()));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_retrieval_presets() {
        let p_turbo = RetrievalPreset::from_str("turbo");
        assert_eq!(p_turbo, RetrievalPreset::Turbo);
        assert_eq!(p_turbo.top_k(), 10);
        assert_eq!(p_turbo.candidate_pool_k(), 50);

        let p_deep = RetrievalPreset::from_str("deep_research");
        assert_eq!(p_deep, RetrievalPreset::Deep);
        assert_eq!(p_deep.top_k(), 40);
        assert_eq!(p_deep.candidate_pool_k(), 150);

        let p_bal = RetrievalPreset::from_str("other");
        assert_eq!(p_bal, RetrievalPreset::Balanced);

        let req = HybridSearchRequest::from_preset(p_turbo, "auth logic", Some("Security".to_string()));
        assert_eq!(req.top_k, 10);
        assert_eq!(req.candidate_pool_k, 50);
        assert_eq!(req.workspace.as_deref(), Some("Security"));
    }

    #[test]
    fn test_pipeline_reranking_fallback_and_scoring() {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_rerank_{}", std::process::id()));
        let mut pipeline = NativeHybridPipeline::open(&temp_dir).unwrap();

        assert_eq!(pipeline.rerank_mode(), "auto");
        pipeline.set_rerank_mode("off");
        assert_eq!(pipeline.rerank_mode(), "off");
        pipeline.set_rerank_mode("on");
        assert_eq!(pipeline.rerank_mode(), "on");

        // Reranker is initially None, so it must fall back to RRF with zero errors
        assert!(pipeline.reranker().is_none());

        // Attach an OnnxReranker pointing to a non-existent path -> is_neural_active is false -> fallback to RRF
        let dummy_reranker = actx_onnx::OnnxReranker::new(std::path::Path::new("missing_weights.onnx"));
        assert!(!dummy_reranker.is_neural_active());
        pipeline.set_reranker(Some(std::sync::Arc::new(dummy_reranker)));

        assert!(pipeline.reranker().is_some());
        assert!(!pipeline.reranker().unwrap().is_neural_active());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    fn result(id: &str, text: &str, score: f64) -> HybridSearchResult {
        HybridSearchResult {
            chunk_id: id.to_string(),
            file_name: format!("{id}.md"),
            file_path: format!("docs/{id}.md"),
            workspace: "Default".to_string(),
            text: text.to_string(),
            content_type: "markdown".to_string(),
            content_hash: String::new(),
            score,
            dense_score: None,
            sparse_score: None,
            token_count: 4,
            matched_subqueries: Vec::new(),
            last_modified: None,
            header_path: None,
            start_line: None,
            end_line: None,
        }
    }

    #[tokio::test]
    async fn test_apply_neural_rerank_without_weights_keeps_rrf_order() {
        let reranker = std::sync::Arc::new(actx_onnx::OnnxReranker::new(std::path::Path::new("missing/model.onnx")));
        let mut items = vec![result("a", "theme colors", 0.03), result("b", "timeout seconds", 0.02)];
        apply_neural_rerank(&reranker, "what timeout", &mut items, &std::collections::HashSet::new()).await;
        assert_eq!(items[0].chunk_id, "a");
        assert_eq!(items[0].score, 0.03);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_apply_neural_rerank_with_fixture_reorders_and_protects() {
        if actx_onnx::runtime::probe_runtime().is_err() {
            if std::env::var("ACTX_REQUIRE_ORT").as_deref() == Ok("1") {
                panic!("ACTX_REQUIRE_ORT=1 but ONNX Runtime could not be loaded");
            }
            eprintln!("skipping: ONNX Runtime library unavailable (set ORT_DYLIB_PATH)");
            return;
        }
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../actx-onnx/tests/fixtures");
        let reranker = std::sync::Arc::new(
            actx_onnx::OnnxReranker::with_paths(
                &fixtures.join("tiny_reranker.onnx"),
                &fixtures.join("tiny_tokenizer.json"),
                "tiny-fixture",
                actx_onnx::ExecutionDevice::Cpu,
            )
            .with_max_length(32),
        );
        let mut items = vec![
            result("theme", "the theme colors", 0.03),
            result("timeout", "timeout is thirty seconds", 0.02),
            result("pinned", "legacy theme", 0.95),
        ];
        let protected: std::collections::HashSet<String> = ["pinned".to_string()].into_iter().collect();
        apply_neural_rerank(&reranker, "what timeout", &mut items, &protected).await;

        let score = |id: &str| items.iter().find(|c| c.chunk_id == id).unwrap().score;
        assert!(score("timeout") > score("theme"), "neural score must lift the timeout chunk");
        assert_eq!(score("pinned"), 0.95, "guaranteed filename matches keep their score");
        assert!(last_rerank_latency_ms().is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_apply_neural_rerank_protected_skip_slots_and_tail_stays_below() {
        if actx_onnx::runtime::probe_runtime().is_err() {
            if std::env::var("ACTX_REQUIRE_ORT").as_deref() == Ok("1") {
                panic!("ACTX_REQUIRE_ORT=1 but ONNX Runtime could not be loaded");
            }
            eprintln!("skipping: ONNX Runtime library unavailable (set ORT_DYLIB_PATH)");
            return;
        }
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../actx-onnx/tests/fixtures");
        let reranker = std::sync::Arc::new(
            actx_onnx::OnnxReranker::with_paths(
                &fixtures.join("tiny_reranker.onnx"),
                &fixtures.join("tiny_tokenizer.json"),
                "tiny-fixture",
                actx_onnx::ExecutionDevice::Cpu,
            )
            .with_max_length(32),
        );
        // Two pinned matches on top, MAX_RERANK_CANDIDATES regular candidates and one tail item.
        let mut items = vec![result("pin-a", "legacy theme", 1.0), result("pin-b", "legacy colors", 1.0)];
        for i in 0..MAX_RERANK_CANDIDATES {
            items.push(result(&format!("c{i}"), "timeout is thirty seconds", 0.5 - i as f64 * 0.001));
        }
        items.push(result("tail", "the theme colors", 0.4));
        let protected: std::collections::HashSet<String> =
            ["pin-a".to_string(), "pin-b".to_string()].into_iter().collect();
        apply_neural_rerank(&reranker, "what timeout", &mut items, &protected).await;

        let score = |id: &str| items.iter().find(|c| c.chunk_id == id).unwrap().score;
        assert_eq!(score("pin-a"), 1.0);
        assert_eq!(score("pin-b"), 1.0);
        // Pinned matches do not consume cross-encoder slots: the last regular candidate is re-scored.
        let last = format!("c{}", MAX_RERANK_CANDIDATES - 1);
        assert_ne!(score(&last), 0.5 - (MAX_RERANK_CANDIDATES - 1) as f64 * 0.001);
        let min_head = (0..MAX_RERANK_CANDIDATES)
            .map(|i| score(&format!("c{i}")))
            .fold(f64::MAX, f64::min);
        assert!(score("tail") < min_head, "unscored tail must stay below every re-scored item");
    }

    /// BM25-only pipeline (no embedding provider): every hit is a single-list hit scored 1/(60 + rank).
    fn bm25_pipeline(name: &str, chunks: &[(&str, &str, &str, &str)]) -> (NativeHybridPipeline, PathBuf) {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let pipeline = NativeHybridPipeline::open(&temp_dir).unwrap();
        for (id, text, file, modified) in chunks {
            pipeline.add_chunk_to_bm25(
                "workspace_chunks",
                id.to_string(),
                text.to_string(),
                file.to_string(),
                format!("/docs/{file}"),
                "Default".to_string(),
                "Local Document".to_string(),
                Some(modified.to_string()),
            );
        }
        (pipeline, temp_dir)
    }

    #[test]
    fn test_bm25_only_hits_get_provenance_from_lancedb() {
        let (pipeline, dir) = bm25_pipeline(
            "provenance_bm25",
            &[("g_0", "kestrel timeouts are retried three times", "guide.md", SAME_DAY), ("n_0", "kestrel notes", "notes.md", SAME_DAY)],
        );
        // Only g_0 has a LanceDB row with provenance; there is no query vector (no provider).
        let record = crate::storage::VectorRecord {
            id: "g_0".into(),
            vector: vec![0.1; 4],
            text: "kestrel timeouts are retried three times".into(),
            file_name: "guide.md".into(),
            file_path: "/docs/guide.md".into(),
            workspace: "Default".into(),
            last_modified: Some(SAME_DAY.into()),
            content_type: Some("markdown".into()),
            document_summary: None,
            keywords: None,
            content_hash: None,
            header_path: Some("guide.md > Timeouts".into()),
            start_line: Some(40),
            end_line: Some(58),
        };
        pipeline.lance_store.upsert_records(vec![record], None, Some(4)).unwrap();

        let results = pipeline
            .search(HybridSearchRequest { query_text: "kestrel".to_string(), ..Default::default() })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let guide = results.iter().find(|r| r.chunk_id == "g_0").expect("guide hit");
        assert_eq!(guide.dense_score, None, "found by BM25 only");
        assert_eq!(guide.header_path.as_deref(), Some("guide.md > Timeouts"));
        assert_eq!((guide.start_line, guide.end_line), (Some(40), Some(58)));
        let notes = results.iter().find(|r| r.chunk_id == "n_0").expect("notes hit");
        assert_eq!((notes.header_path.as_deref(), notes.start_line), (None, None));
    }

    fn stored_row(id: &str, text: &str, file: &str, vector: Vec<f32>) -> crate::storage::VectorRecord {
        crate::storage::VectorRecord {
            id: id.into(),
            vector,
            text: text.into(),
            file_name: file.into(),
            file_path: format!("/docs/{file}"),
            workspace: "Default".into(),
            last_modified: Some(SAME_DAY.into()),
            content_type: Some("markdown".into()),
            document_summary: None,
            keywords: None,
            content_hash: Some(format!("sha-of-{file}")),
            header_path: None,
            start_line: None,
            end_line: None,
        }
    }

    /// Regression BUG-023: `Default` searched without a workspace filter, so its answers mixed in
    /// chunks of every other workspace through dense search, filename metadata and BM25. It now
    /// sees only its own chunks and `Global`, with or without the workspace named in the request.
    #[test]
    fn test_default_workspace_does_not_see_other_workspaces() {
        let (mut pipeline, dir) = bm25_pipeline("default_isolation", &[("own_0", "kestrel timeouts in the own guide", "guide.md", SAME_DAY)]);
        pipeline.add_chunk_to_bm25(
            "workspace_chunks", "other_0".into(), "kestrel timeouts in the other guide".into(), "other.md".into(),
            "/other/other.md".into(), "Other".into(), "Local Document".into(), Some(SAME_DAY.into()),
        );
        pipeline.add_chunk_to_bm25(
            "workspace_chunks", "global_0".into(), "kestrel timeouts in the AnyContext guide".into(), "README.md".into(),
            "system://README.md".into(), "Global".into(), "System Documentation".into(), Some(SAME_DAY.into()),
        );
        let own = stored_row("own_0", "kestrel timeouts in the own guide", "guide.md", vec![1.0, 0.0, 0.0, 0.0]);
        let other = crate::storage::VectorRecord {
            workspace: "Other".into(),
            file_path: "/other/other.md".into(),
            ..stored_row("other_0", "kestrel timeouts in the other guide", "other.md", vec![1.0, 0.0, 0.0, 0.0])
        };
        let global = crate::storage::VectorRecord {
            workspace: "Global".into(),
            file_path: "system://README.md".into(),
            ..stored_row("global_0", "kestrel timeouts in the AnyContext guide", "README.md", vec![1.0, 0.0, 0.0, 0.0])
        };
        pipeline.lance_store.upsert_records(vec![own, other, global], None, Some(4)).unwrap();
        pipeline.set_global_dense(true);

        for workspace in [Some("Default".to_string()), None] {
            // "other.md" is a filename mention, so the metadata path is exercised too.
            let results = pipeline
                .search(HybridSearchRequest {
                    query_text: "kestrel timeouts other.md".to_string(),
                    query_vector: Some(vec![1.0, 0.0, 0.0, 0.0]),
                    workspace: workspace.clone(),
                    top_k: 8,
                    candidate_pool_k: 30,
                    min_score: RRF_MIN_SCORE,
                    ..Default::default()
                })
                .unwrap();
            let ids: Vec<&str> = results.iter().map(|r| r.chunk_id.as_str()).collect();
            assert!(ids.contains(&"own_0"), "{workspace:?}: {ids:?}");
            assert!(ids.contains(&"global_0"), "{workspace:?}: Global stays visible: {ids:?}");
            assert!(!ids.contains(&"other_0"), "{workspace:?}: another workspace leaked: {ids:?}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    const ENVELOPED: &str =
        "[Context: Document 'guide.md': Kestrel is a gateway. | Keywords: kestrel, gateway]\n---\nkestrel timeouts are retried three times";

    #[test]
    fn test_bm25_only_hits_take_raw_text_from_lancedb() {
        let (pipeline, dir) = bm25_pipeline("raw_text_bm25", &[("g_0", ENVELOPED, "guide.md", SAME_DAY)]);
        let row = stored_row("g_0", "kestrel timeouts are retried three times", "guide.md", vec![0.1; 4]);
        pipeline.lance_store.upsert_records(vec![row], None, Some(4)).unwrap();

        let results = pipeline
            .search(HybridSearchRequest { query_text: "kestrel".to_string(), ..Default::default() })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let guide = results.iter().find(|r| r.chunk_id == "g_0").expect("guide hit");
        assert_eq!(guide.dense_score, None, "found by BM25 only");
        assert_eq!(guide.text, "kestrel timeouts are retried three times", "same text as a dense hit");
        assert_eq!(guide.content_hash, "sha-of-guide.md");
    }

    #[test]
    fn test_bm25_only_hits_without_a_row_drop_the_envelope() {
        let (pipeline, dir) = bm25_pipeline("strip_bm25", &[("g_0", ENVELOPED, "guide.md", SAME_DAY)]);
        let results = pipeline
            .search(HybridSearchRequest { query_text: "kestrel".to_string(), ..Default::default() })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "kestrel timeouts are retried three times");
        assert_eq!(results[0].content_hash, "");
    }

    /// Dense-only pipeline: LanceDB rows of one file (same file-level content hash), no BM25.
    fn dense_pipeline(name: &str, ids: &[&str]) -> (NativeHybridPipeline, PathBuf) {
        let (pipeline, dir) = bm25_pipeline(name, &[]);
        let rows = ids
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let mut v = vec![0.1; 4];
                v[i % 4] = 1.0;
                stored_row(id, &format!("alpha section {i} about falcons"), "alpha.md", v)
            })
            .collect();
        pipeline.lance_store.upsert_records(rows, None, Some(4)).unwrap();
        (pipeline, dir)
    }

    fn sub_query(id: &str) -> HybridSearchRequest {
        HybridSearchRequest {
            sub_query_id: Some(id.to_string()),
            query_text: "zzz".to_string(),
            query_vector: Some(vec![1.0, 0.6, 0.3, 0.1]),
            ..Default::default()
        }
    }

    #[test]
    fn test_batch_keeps_every_chunk_of_a_file() {
        let (pipeline, dir) = dense_pipeline("dedup_file", &["a_0", "a_1", "a_2"]);
        let single = pipeline.search(sub_query("q1")).unwrap();
        let results = pipeline.retrieve_hybrid_batch(vec![sub_query("q1"), sub_query("q2")]).unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let mut ids: Vec<&str> = results.iter().map(|r| r.chunk_id.as_str()).collect();
        ids.sort();
        assert_eq!(ids, vec!["a_0", "a_1", "a_2"], "chunks sharing the file hash stay distinct");
        for r in &results {
            let own = single.iter().find(|s| s.chunk_id == r.chunk_id).unwrap().score;
            assert!((r.score - 2.0 * own).abs() < 1e-12, "{} carries only its own scores", r.chunk_id);
        }
    }

    #[test]
    fn test_batch_merges_the_same_chunk_across_sub_queries() {
        let (pipeline, dir) = dense_pipeline("dedup_same", &["a_0"]);
        let results = pipeline.retrieve_hybrid_batch(vec![sub_query("q1"), sub_query("q2")]).unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matched_subqueries, vec!["q1".to_string(), "q2".to_string()]);
    }

    fn assert_descending(results: &[HybridSearchResult]) {
        let order: Vec<(&str, f64)> = results.iter().map(|r| (r.chunk_id.as_str(), r.score)).collect();
        assert!(
            results.windows(2).all(|w| w[0].score >= w[1].score),
            "results must be in descending score order: {order:?}"
        );
    }

    const SAME_DAY: &str = "2026-10-10T12:00:00Z";

    fn two_source_chunks() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
        vec![
            ("a1", "kestrel kestrel kestrel kestrel nesting", "alpha.md", SAME_DAY),
            ("a2", "kestrel kestrel kestrel nesting habits", "alpha.md", SAME_DAY),
            ("a3", "kestrel kestrel nesting habits of birds", "alpha.md", SAME_DAY),
            ("b1", "kestrel nesting habits of small birds of prey", "beta.md", SAME_DAY),
        ]
    }

    #[test]
    fn search_single_returns_descending_scores_across_sources() {
        let (pipeline, dir) = bm25_pipeline("order_single", &two_source_chunks());
        let results = pipeline
            .search(HybridSearchRequest { query_text: "kestrel".to_string(), ..Default::default() })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results.len(), 4);
        assert_descending(&results);
        assert_eq!(results[0].chunk_id, "a1");
        assert_eq!(results[1].chunk_id, "a2", "the second best chunk stays second");
    }

    #[test]
    fn batch_returns_descending_scores() {
        let (pipeline, dir) = bm25_pipeline("order_batch", &two_source_chunks());
        let results = pipeline
            .retrieve_hybrid_batch(vec![HybridSearchRequest { query_text: "kestrel".to_string(), ..Default::default() }])
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results.len(), 4);
        assert_descending(&results);
        assert_eq!(results[1].chunk_id, "a2");
    }

    #[test]
    fn recency_does_not_outrank_relevance() {
        let today = chrono::Utc::now().to_rfc3339();
        let (pipeline, dir) = bm25_pipeline(
            "recency",
            &[
                ("old", "osprey osprey osprey migration", "old.md", "2020-01-01T00:00:00Z"),
                ("new", "osprey migration routes over the northern coast", "new.md", today.as_str()),
            ],
        );
        let results = pipeline
            .search(HybridSearchRequest { query_text: "osprey".to_string(), ..Default::default() })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results[0].chunk_id, "old", "the better match wins regardless of age");
        assert_eq!(results[0].score, 1.0 / 61.0, "scores are plain RRF scores");
        assert_eq!(results[1].score, 1.0 / 62.0);
    }

    fn dated(id: &str, score: f64, modified: Option<&str>) -> HybridSearchResult {
        HybridSearchResult { last_modified: modified.map(str::to_string), ..result(id, id, score) }
    }

    #[test]
    fn compare_by_relevance_uses_recency_only_on_ties() {
        let mut items = vec![
            dated("new-weak", 0.010, Some("2026-10-16T00:00:00Z")),
            dated("old-strong", 0.020, Some("2020-01-01T00:00:00Z")),
            dated("no-date", 0.015, None),
            dated("tie-invalid", 0.015, Some("yesterday")),
            dated("tie-old", 0.015, Some("2021-01-01T00:00:00+02:00")),
            dated("tie-new", 0.015, Some("2026-01-01T00:00:00Z")),
        ];
        items.sort_by(compare_by_relevance);
        let order: Vec<&str> = items.iter().map(|c| c.chunk_id.as_str()).collect();
        assert_eq!(order, vec!["old-strong", "tie-new", "tie-old", "no-date", "tie-invalid", "new-weak"]);
    }

    #[test]
    fn rrf_min_score_keeps_whole_pool() {
        assert!(1.0 / (60.0 + 140.0) >= RRF_MIN_SCORE);
    }

    #[test]
    fn batch_keeps_single_list_hits_at_rrf_min_score() {
        let (pipeline, dir) = bm25_pipeline(
            "min_score",
            &[("only-bm25", "heron heron wading in the shallows", "heron.md", SAME_DAY)],
        );
        let results = pipeline
            .retrieve_hybrid_batch(vec![HybridSearchRequest {
                query_text: "heron".to_string(),
                min_score: RRF_MIN_SCORE,
                ..Default::default()
            }])
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results.len(), 1, "a hit found by one list only must survive the floor");
        assert_eq!(results[0].score, 1.0 / 61.0);
    }
    /// Embedding provider with a fixed answer, for the query embedding outcome.
    struct FixedEmbedder(Result<Vec<Vec<f32>>, ()>);

    #[async_trait::async_trait]
    impl LmProvider for FixedEmbedder {
        fn provider_id(&self) -> &'static str {
            "fixed"
        }

        async fn chat_complete(&self, _request: actx_lm::types::ChatRequest) -> Result<actx_lm::types::ChatResponse, actx_lm::LmError> {
            Err(actx_lm::LmError::Unsupported("chat".into()))
        }

        async fn chat_stream(&self, _request: actx_lm::types::ChatRequest) -> Result<actx_lm::traits::BoxedChunkStream, actx_lm::LmError> {
            Err(actx_lm::LmError::Unsupported("chat".into()))
        }

        async fn embed(&self, _model: &str, _texts: &[String]) -> Result<Vec<Vec<f32>>, actx_lm::LmError> {
            self.0.clone().map_err(|_| actx_lm::LmError::Auth { provider: "fixed".into(), message: "invalid key".into() })
        }
    }

    fn query_outcome(name: &str, provider: Option<FixedEmbedder>, query_vector: Option<Vec<f32>>) -> Option<QueryEmbeddingOutcome> {
        let (mut pipeline, dir) = bm25_pipeline(name, &[("k_0", "kestrel timeouts are retried three times", "guide.md", SAME_DAY)]);
        pipeline.set_embedding(provider.map(|p| Arc::new(p) as Arc<dyn LmProvider>), "fixed-embed");
        let results = pipeline
            .search(HybridSearchRequest { query_text: "kestrel".to_string(), query_vector, ..Default::default() })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);
        assert!(results.iter().any(|r| r.chunk_id == "k_0"), "BM25 still answers");
        pipeline.last_query_embedding()
    }

    #[test]
    fn test_query_embedding_failure_is_recorded() {
        let outcome = query_outcome("qe_failed", Some(FixedEmbedder(Err(()))), None);
        match outcome {
            Some(QueryEmbeddingOutcome::Failed(reason)) => assert!(reason.contains("invalid key"), "{reason}"),
            other => panic!("expected a failure, got {other:?}"),
        }
        let empty = query_outcome("qe_empty", Some(FixedEmbedder(Ok(vec![]))), None);
        assert_eq!(empty, Some(QueryEmbeddingOutcome::Failed("empty embedding answer".to_string())));
        assert!(last_query_embedding_outcome().is_some(), "the process-wide outcome is recorded too");
    }

    #[test]
    fn test_query_without_provider_is_recorded() {
        assert_eq!(query_outcome("qe_none", None, None), Some(QueryEmbeddingOutcome::NoProvider));
    }

    #[test]
    fn test_query_dense_outcome_is_recorded() {
        assert_eq!(query_outcome("qe_vector", None, Some(vec![0.1; 4])), Some(QueryEmbeddingOutcome::Dense));
        assert_eq!(query_outcome("qe_embedded", Some(FixedEmbedder(Ok(vec![vec![0.1; 4]]))), None), Some(QueryEmbeddingOutcome::Dense));
    }

    /// BUG-021: a 768-value query (Ollama `nomic-embed-text`) against the 1536 table, whose rows
    /// were stored zero-padded, must reach dense search instead of failing silently.
    #[test]
    fn test_short_query_vector_reaches_dense_search() {
        let short = crate::storage::lancedb::DEFAULT_VECTOR_DIM / 2;
        let (pipeline, dir) = bm25_pipeline("short_query", &[("k_0", "kestrel timeouts are retried three times", "guide.md", SAME_DAY)]);
        let mut near = vec![0.0; short];
        near[0] = 1.0;
        let mut far = vec![0.0; short];
        far[1] = 1.0;
        let rows = vec![
            stored_row("d_near", "pasta recipe with basil", "near.md", near.clone()),
            stored_row("d_far", "notes about herons", "far.md", far),
        ];
        pipeline.lance_store.upsert_records(rows, None, None).unwrap();

        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "kestrel".to_string(),
                query_vector: Some(near),
                min_score: RRF_MIN_SCORE,
                ..Default::default()
            })
            .unwrap();
        let outcome = pipeline.last_query_embedding();
        let _ = std::fs::remove_dir_all(dir);

        let near_hit = results.iter().find(|r| r.chunk_id == "d_near").expect("dense search finds the near chunk");
        assert!(near_hit.dense_score.is_some());
        assert_eq!(outcome, Some(QueryEmbeddingOutcome::Dense));
    }

    #[test]
    fn test_dense_search_failure_is_recorded() {
        let (pipeline, dir) = bm25_pipeline("dense_failed", &[("k_0", "kestrel timeouts are retried three times", "guide.md", SAME_DAY)]);
        pipeline
            .lance_store
            .upsert_records(vec![stored_row("d_0", "pasta recipe", "pasta.md", vec![0.1; 4])], None, Some(4))
            .unwrap();

        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "kestrel".to_string(),
                query_vector: Some(vec![0.1; 8]),
                ..Default::default()
            })
            .unwrap();
        let outcome = pipeline.last_query_embedding();
        let _ = std::fs::remove_dir_all(dir);

        assert!(results.iter().any(|r| r.chunk_id == "k_0"), "BM25 still answers");
        match outcome {
            Some(QueryEmbeddingOutcome::SearchFailed(reason)) => assert!(reason.contains("8 values"), "{reason}"),
            other => panic!("expected a dense search failure, got {other:?}"),
        }
    }

    #[test]
    fn test_query_embedding_outcome_descriptions() {
        assert_eq!(QueryEmbeddingOutcome::Dense.describe(), "dense + BM25 (last query)");
        assert_eq!(QueryEmbeddingOutcome::NoProvider.describe(), "BM25 only (last query): no embedding provider");
        assert_eq!(
            QueryEmbeddingOutcome::NoEmbeddingKey.describe(),
            "BM25 only (last query): no embedding API key (/keys)"
        );
        assert_eq!(
            QueryEmbeddingOutcome::Failed("quota".into()).describe(),
            "BM25 only (last query): query embedding failed: quota"
        );
        assert_eq!(
            QueryEmbeddingOutcome::SearchFailed("size".into()).describe(),
            "BM25 only (last query): dense search failed: size"
        );
    }
    /// Regression BUG-019: on the mock target every chunk and query vector is the same, so dense
    /// search returned an arbitrary pool and unrelated chunks reached the results. With the query
    /// embedding the CLI resolves, only the keyword match is returned and the outcome says why.
    #[test]
    fn test_constant_mock_vectors_do_not_rank_unrelated_chunks() {
        let keys = ["OPENAI_API_KEY", "GEMINI_API_KEY", "ACTX_EMBEDDING_MODEL", "ACTX_MODEL"];
        if keys.iter().any(|v| std::env::var(v).map(|s| !s.trim().is_empty()).unwrap_or(false)) {
            eprintln!("skipped: one of {keys:?} is set");
            return;
        }
        let db = crate::storage::NativeConfigDb::open_in_memory().expect("db");
        let (provider, target) = crate::ingestion::resolve_query_embedding_provider(&db, "Default");
        assert!(target.is_mock(), "{}", target.key());

        let mut chunks = vec![("k_0", "kestrel timeouts are retried three times", "guide.md", SAME_DAY)];
        // One file per chunk, so file-level deduplication cannot hide them.
        let unrelated: Vec<(String, String, String)> = (0..6)
            .map(|i| (format!("u_{i}"), format!("pasta recipe number {i} with basil"), format!("recipe_{i}.md")))
            .collect();
        for (id, text, file) in &unrelated {
            chunks.push((id.as_str(), text.as_str(), file.as_str(), SAME_DAY));
        }
        let (mut pipeline, dir) = bm25_pipeline("mock_constant", &chunks);
        // The vectors ingestion stores on the mock target (same size as the mock query vector).
        let dim = crate::storage::lancedb::DEFAULT_VECTOR_DIM;
        let rows = chunks
            .iter()
            .map(|(id, text, file, _)| stored_row(id, text, file, vec![0.1; dim]))
            .collect();
        pipeline.lance_store.upsert_records(rows, None, Some(dim)).unwrap();
        pipeline.set_query_embedding(provider, &target);

        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "kestrel timeouts".to_string(),
                top_k: 8,
                candidate_pool_k: 30,
                min_score: RRF_MIN_SCORE,
                ..Default::default()
            })
            .unwrap();
        let outcome = pipeline.last_query_embedding();
        let _ = std::fs::remove_dir_all(dir);

        let ids: Vec<&str> = results.iter().map(|r| r.chunk_id.as_str()).collect();
        assert_eq!(ids, vec!["k_0"], "no unrelated chunk may come from dense search");
        assert_eq!(results[0].dense_score, None);
        assert_eq!(outcome, Some(QueryEmbeddingOutcome::NoEmbeddingKey));
    }
    /// `Global` rows as the startup bootstrap stores them (placeholder vectors) or as a sync
    /// stores them once embedded, next to `Proj` rows with real-looking vectors.
    fn global_and_project_pipeline(name: &str, global_vector: Vec<f32>) -> (NativeHybridPipeline, PathBuf) {
        let (pipeline, dir) = bm25_pipeline(name, &[]);
        let mut rows = vec![stored_row("k_0", "kestrel timeouts are retried three times", "guide.md", vec![1.0, 0.0, 0.0, 0.0])];
        rows.push(stored_row("g_kw", "kestrel gateway commands of AnyContext", "SYSTEM_KNOWLEDGE.md", global_vector.clone()));
        for i in 0..3 {
            rows.push(stored_row(&format!("global_sys_doc_{i}"), &format!("slash command help {i}"), "README.md", global_vector.clone()));
        }
        for row in rows.iter_mut() {
            row.workspace = if row.file_name == "guide.md" { "Proj".into() } else { "Global".into() };
            row.file_path = format!("/docs/{}/{}", row.workspace, row.id);
            pipeline.add_chunk_to_bm25(
                "workspace_chunks", row.id.clone(), row.text.clone(), row.file_name.clone(), row.file_path.clone(),
                row.workspace.clone(), "Local Document".to_string(), Some(SAME_DAY.to_string()),
            );
        }
        pipeline.lance_store.upsert_records(rows, None, Some(4)).unwrap();
        (pipeline, dir)
    }

    /// `Default` is filtered like any other workspace (BUG-023): it sees `Global`, not `Proj`.
    fn global_request(workspace: &str) -> HybridSearchRequest {
        HybridSearchRequest {
            query_text: "kestrel timeouts".to_string(),
            query_vector: Some(vec![0.8, 0.6, 0.0, 0.0]),
            workspace: Some(workspace.to_string()),
            target_workspaces: vec!["Global".to_string()],
            top_k: 8,
            candidate_pool_k: 30,
            min_score: RRF_MIN_SCORE,
            ..Default::default()
        }
    }

    /// Regression BUG-019: `Global` rows hold placeholder (zero) vectors, all at the same distance
    /// from any query, so they reached the results through dense search alone. Without a matching
    /// target they are found by BM25 and metadata only, on a named workspace and on `Default`.
    #[test]
    fn test_global_rows_are_excluded_from_dense_search_without_a_matching_target() {
        for workspace in ["Proj", "Default"] {
            let (pipeline, dir) = global_and_project_pipeline("global_gate_off", vec![0.0; 4]);
            let results = pipeline.search(global_request(workspace)).unwrap();
            let _ = std::fs::remove_dir_all(dir);

            let ids: Vec<&str> = results.iter().map(|r| r.chunk_id.as_str()).collect();
            assert!(!ids.iter().any(|id| id.starts_with("global_sys_doc")), "{workspace:?}: {ids:?}");
            let keyword = results.iter().find(|r| r.chunk_id == "g_kw").expect("BM25 still finds Global");
            assert_eq!(keyword.dense_score, None, "{workspace:?}");
            let own = results.iter().any(|r| r.chunk_id == "k_0" && r.dense_score.is_some());
            assert_eq!(own, workspace == "Proj", "{workspace:?}: Proj's chunk is found only from Proj (BUG-023)");
        }
    }

    /// Once `Global` is embedded with the query's target, its rows take part in dense search.
    #[test]
    fn test_global_rows_take_part_in_dense_search_when_enabled() {
        for workspace in ["Proj", "Default"] {
            let (mut pipeline, dir) = global_and_project_pipeline("global_gate_on", vec![0.8, 0.6, 0.0, 0.0]);
            pipeline.set_global_dense(true);
            let results = pipeline.search(global_request(workspace)).unwrap();
            let _ = std::fs::remove_dir_all(dir);

            assert!(
                results.iter().any(|r| r.chunk_id.starts_with("global_sys_doc") && r.dense_score.is_some()),
                "{workspace:?}"
            );
        }
    }
}
