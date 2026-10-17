//! Native Hybrid RAG Pipeline and RFC-042 Batch Retrieval Engine in Rust.
//!
//! Unifies dense vector similarity (LanceDB), Okapi BM25 lexical ranking,
//! Reciprocal Rank Fusion (RRF), cross-query SHA-256 deduplication with score accumulation,
//! source-fair diversification, and density budgeting entirely in native Rust.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::storage::lancedb::{NativeLanceStore, ScoredVectorResult};
use crate::retrieval::bm25::BM25Index;
use crate::retrieval::rrf::reciprocal_rank_fusion;
use crate::retrieval::diversifier::{RankedChunk, apply_source_diversification, apply_density_budget};
use crate::retrieval::query::QueryPreprocessor;
use crate::retrieval::token_budget::estimate_token_count;
use crate::embedding::resolve_embedding_target;
use crate::storage::sqlite::NativeConfigDb;

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
}

/// Maximum candidates re-scored by the cross-encoder per query (ADR-130).
pub const MAX_RERANK_CANDIDATES: usize = 30;

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

/// Port: embeds a query in the embedding space a workspace was indexed with.
#[async_trait::async_trait]
pub trait QueryEmbedder: Send + Sync {
    /// Identity of the workspace's embedding space; workspaces sharing it share one query vector.
    fn space_key(&self, workspace: &str) -> String;

    /// Embeds `text` with the workspace's embedding provider and model.
    async fn embed_query(&self, workspace: &str, text: &str) -> Result<Vec<f32>, String>;
}

/// Default [`QueryEmbedder`]: resolves each workspace's embedding target exactly as ingestion does.
pub struct WorkspaceQueryEmbedder {
    db: Arc<NativeConfigDb>,
}

impl WorkspaceQueryEmbedder {
    pub fn new(db: Arc<NativeConfigDb>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl QueryEmbedder for WorkspaceQueryEmbedder {
    fn space_key(&self, workspace: &str) -> String {
        let (provider, model) = resolve_embedding_target(&self.db, None, workspace).space_key();
        format!("{provider}/{model}")
    }

    async fn embed_query(&self, workspace: &str, text: &str) -> Result<Vec<f32>, String> {
        let target = resolve_embedding_target(&self.db, None, workspace);
        let vecs = target
            .provider
            .embed(&target.model, &[text.to_string()])
            .await
            .map_err(|e| e.to_string())?;
        vecs.into_iter()
            .next()
            .filter(|v| !v.is_empty())
            .ok_or_else(|| "provider returned an empty embedding".to_string())
    }
}

/// Outcome of the most recent query embedding in this process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryEmbeddingStatus {
    /// The query was embedded in `space`.
    Embedded { space: String },
    /// Embedding failed; the workspaces of `space` fell back to BM25 only (ADR-110).
    Failed { space: String, error: String },
}

static LAST_QUERY_EMBEDDING: std::sync::Mutex<Option<QueryEmbeddingStatus>> = std::sync::Mutex::new(None);

/// Outcome of the most recent query embedding in this process, if any.
pub fn last_query_embedding_status() -> Option<QueryEmbeddingStatus> {
    LAST_QUERY_EMBEDDING.lock().ok().and_then(|s| s.clone())
}

fn record_query_embedding(status: QueryEmbeddingStatus) {
    if let Ok(mut slot) = LAST_QUERY_EMBEDDING.lock() {
        *slot = Some(status);
    }
}

/// Unified Native Hybrid Retrieval and Reranking Pipeline.
pub struct NativeHybridPipeline {
    db_path: PathBuf,
    lance_store: Arc<NativeLanceStore>,
    bm25_indices: Arc<RwLock<HashMap<String, BM25Index>>>,
    query_embedder: Option<Arc<dyn QueryEmbedder>>,
    reranker: Option<Arc<actx_onnx::OnnxReranker>>,
    rerank_mode: String,
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
            query_embedder: None,
            reranker: None,
            rerank_mode: "auto".to_string(),
        })
    }

    /// Creates pipeline with an existing [`NativeLanceStore`] and optional [`QueryEmbedder`].
    pub fn new(lance_store: Arc<NativeLanceStore>, query_embedder: Option<Arc<dyn QueryEmbedder>>) -> Self {
        let p = lance_store.db_path().to_path_buf();
        Self {
            db_path: p,
            lance_store,
            bm25_indices: Arc::new(RwLock::new(HashMap::new())),
            query_embedder,
            reranker: None,
            rerank_mode: "auto".to_string(),
        }
    }

    /// Sets the optional query embedder used for dense search.
    pub fn set_query_embedder(&mut self, query_embedder: Option<Arc<dyn QueryEmbedder>>) {
        self.query_embedder = query_embedder;
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
        let mut targets = Vec::new();
        if let Some(ws) = &req.workspace {
            if !ws.is_empty() {
                targets.push((ws.clone(), req.candidate_pool_k));
            }
        }
        for tw in &req.target_workspaces {
            if !tw.is_empty()
                && (req.workspace.as_ref() != Some(tw))
                && !targets.iter().any(|(w, _)| w == tw)
            {
                targets.push((tw.clone(), std::cmp::max(10, req.candidate_pool_k / 2)));
            }
        }
        if targets.is_empty() {
            targets.push(("Default".to_string(), req.candidate_pool_k));
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
                    let ws_filter = if ws_name == "Default" {
                        None
                    } else {
                        Some(ws_name.as_str())
                    };
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
                let ws_filter = if ws_name == "Default" {
                    None
                } else {
                    Some(ws_name.as_str())
                };
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

        // 5. Query Vector Resolution: passed in, or embedded in each workspace's own embedding space
        let mut space_vectors: HashMap<String, Option<Vec<f32>>> = HashMap::new();
        let mut target_vectors = Vec::with_capacity(targets.len());
        for (ws_name, limit) in &targets {
            let query_vec = if let Some(ref v) = req.query_vector {
                Some(v.clone())
            } else if let Some(ref embedder) = self.query_embedder {
                let space = embedder.space_key(ws_name);
                if let Some(cached) = space_vectors.get(&space) {
                    cached.clone()
                } else {
                    // ADR-110: a failed embedding leaves this workspace on BM25 only, but is recorded.
                    let embedded = match embedder.embed_query(ws_name, &req.query_text).await {
                        Ok(v) => {
                            record_query_embedding(QueryEmbeddingStatus::Embedded { space: space.clone() });
                            Some(v)
                        }
                        Err(error) => {
                            record_query_embedding(QueryEmbeddingStatus::Failed { space: space.clone(), error });
                            None
                        }
                    };
                    space_vectors.insert(space, embedded.clone());
                    embedded
                }
            } else {
                None
            };
            target_vectors.push((ws_name, *limit, query_vec));
        }

        // 6. Dense Vector Similarity Search
        for (ws_name, limit, query_vec) in target_vectors {
            if let Some(q_vec) = query_vec {
                let ws_filter = if ws_name == "Default" {
                    None
                } else {
                    Some(ws_name.as_str())
                };
                if let Ok(matches) = self
                    .lance_store
                    .search_vector_async(q_vec, limit, ws_filter, None, Some(&req.table_name))
                    .await
                {
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
        let target_ws = match &req.workspace {
            Some(w) if w != "Default" => Some(w.as_str()),
            _ => None,
        };
        let sparse_matches = bm25_index.search(&expanded_query, req.candidate_pool_k, target_ws);
        let sparse_ids: Vec<String> = sparse_matches.iter().map(|(id, _)| id.clone()).collect();
        let sparse_score_map: HashMap<String, f32> = sparse_matches.into_iter().collect();

        // 9. Reciprocal Rank Fusion (RRF)
        let fused_results = reciprocal_rank_fusion(&dense_ids, &sparse_ids, Some(req.rrf_k));

        let sub_query_tag = req
            .sub_query_id
            .clone()
            .unwrap_or_else(|| req.query_text.clone());

        // 10. Assemble HybridSearchResult candidates
        let mut results = Vec::with_capacity(fused_results.len());
        for res in fused_results {
            let chunk_id = res.id;
            let dense_entry = dense_map.remove(&chunk_id);
            let sparse_score = sparse_score_map.get(&chunk_id).copied();

            let (file_name, file_path, workspace, text, content_type, content_hash, dense_score, last_modified) =
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
                    )
                } else if let Some(doc) = bm25_index.get_doc_by_id(&chunk_id) {
                    (
                        doc.file_name.clone(),
                        doc.file_path.clone(),
                        doc.workspace.clone(),
                        doc.text.clone(),
                        doc.content_type.clone(),
                        String::new(),
                        None,
                        doc.last_modified.clone(),
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

            // Subtle recency boost: recent chunks receive a micro-boost to cleanly break score ties
            let recency_boost = if let Some(ref lm) = last_modified {
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(lm) {
                    let now = chrono::Utc::now();
                    let days_old = (now - dt.with_timezone(&chrono::Utc)).num_days().max(0) as f64;
                    0.01 / (1.0 + (days_old / 30.0))
                } else {
                    0.0
                }
            } else {
                0.0
            };

            let final_score = if is_guaranteed { 1.0 } else { res.score + recency_boost };

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
            });
        }

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
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Source-fair diversification
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

        let diversified_ranked = apply_source_diversification(
            ranked_chunks,
            req.max_chunks_per_source,
            req.top_k,
        );

        // Density budgeting
        let budgeted_ranked = apply_density_budget(diversified_ranked, req.max_density_chars);

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

    /// RFC-042: Concurrent multi-query batch retrieval with cross-query SHA-256 deduplication,
    /// RRF score accumulation, source-fair diversification, and density budgeting.
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
                let key = if !chunk.content_hash.is_empty() {
                    chunk.content_hash.clone()
                } else if !chunk.chunk_id.is_empty() {
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
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 4. Source-Fair Round-Robin Diversification
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

        let diversified_ranked = apply_source_diversification(
            ranked_chunks,
            first_req.max_chunks_per_source,
            first_req.top_k,
        );

        // 5. Density Budgeting
        let budgeted_ranked =
            apply_density_budget(diversified_ranked, first_req.max_density_chars);

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

    /// Test double: each workspace has its own embedding space and query vector.
    struct SpaceEmbedder {
        spaces: HashMap<&'static str, (&'static str, Result<Vec<f32>, String>)>,
        calls: std::sync::atomic::AtomicUsize,
    }

    #[async_trait::async_trait]
    impl QueryEmbedder for SpaceEmbedder {
        fn space_key(&self, workspace: &str) -> String {
            self.spaces[workspace].0.to_string()
        }

        async fn embed_query(&self, workspace: &str, _text: &str) -> Result<Vec<f32>, String> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.spaces[workspace].1.clone()
        }
    }

    fn record(id: &str, workspace: &str, vector: Vec<f32>) -> crate::storage::lancedb::VectorRecord {
        crate::storage::lancedb::VectorRecord {
            id: id.to_string(),
            vector,
            text: format!("chunk {id}"),
            file_name: format!("{id}.md"),
            file_path: format!("docs/{id}.md"),
            workspace: workspace.to_string(),
            last_modified: None,
            content_type: Some("markdown".to_string()),
            document_summary: None,
            keywords: None,
            content_hash: None,
        }
    }

    fn dense_scores(results: &[HybridSearchResult]) -> HashMap<String, f32> {
        results
            .iter()
            .filter_map(|r| r.dense_score.map(|s| (r.chunk_id.clone(), s)))
            .collect()
    }

    /// BUG-001 regression: each workspace is searched with a query vector from its own embedding
    /// space, one embedding per space, and a failed embedding only drops that workspace to BM25.
    #[test]
    fn test_dense_search_embeds_query_per_workspace_space() {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_query_space_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let mut pipeline = NativeHybridPipeline::open(&temp_dir).unwrap();
        pipeline.set_rerank_mode("off");
        pipeline
            .lance_store
            .upsert_records(
                vec![
                    record("a1", "Alpha", vec![1.0, 0.0, 0.0, 0.0]),
                    record("a2", "Alpha", vec![0.0, 1.0, 0.0, 0.0]),
                    record("b1", "Beta", vec![0.0, 0.0, 1.0, 0.0]),
                    record("b2", "Beta", vec![0.0, 0.0, 0.0, 1.0]),
                ],
                Some("workspace_chunks"),
                Some(4),
            )
            .unwrap();

        let req = HybridSearchRequest {
            query_text: "where is the configuration".to_string(),
            workspace: Some("Alpha".to_string()),
            target_workspaces: vec!["Beta".to_string(), "Gamma".to_string()],
            top_k: 10,
            candidate_pool_k: 10,
            rerank_mode: Some("off".to_string()),
            ..Default::default()
        };

        let embedder = Arc::new(SpaceEmbedder {
            spaces: HashMap::from([
                ("Alpha", ("space-a", Ok(vec![1.0, 0.0, 0.0, 0.0]))),
                ("Gamma", ("space-a", Ok(vec![1.0, 0.0, 0.0, 0.0]))),
                ("Beta", ("space-b", Ok(vec![0.0, 0.0, 1.0, 0.0]))),
            ]),
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        pipeline.set_query_embedder(Some(embedder.clone()));
        let scores = dense_scores(&pipeline.search(req.clone()).unwrap());
        assert!(scores["a1"] > scores["a2"], "Alpha must use its own query vector: {scores:?}");
        assert!(scores["b1"] > scores["b2"], "Beta must use its own query vector: {scores:?}");
        assert_eq!(embedder.calls.load(std::sync::atomic::Ordering::SeqCst), 2, "one embedding per space");

        let failing = Arc::new(SpaceEmbedder {
            spaces: HashMap::from([
                ("Alpha", ("space-a", Ok(vec![1.0, 0.0, 0.0, 0.0]))),
                ("Gamma", ("space-a", Ok(vec![1.0, 0.0, 0.0, 0.0]))),
                ("Beta", ("space-b", Err("embedding unsupported".to_string()))),
            ]),
            calls: std::sync::atomic::AtomicUsize::new(0),
        });
        pipeline.set_query_embedder(Some(failing));
        let scores = dense_scores(&pipeline.search(req).unwrap());
        assert!(scores.contains_key("a1"), "Alpha keeps dense search: {scores:?}");
        assert!(!scores.contains_key("b1") && !scores.contains_key("b2"), "Beta falls back to BM25: {scores:?}");
        assert_eq!(
            last_query_embedding_status(),
            Some(QueryEmbeddingStatus::Failed {
                space: "space-b".to_string(),
                error: "embedding unsupported".to_string(),
            })
        );

        drop(pipeline);
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
