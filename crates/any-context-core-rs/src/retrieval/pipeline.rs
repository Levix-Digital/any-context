//! Native hybrid retrieval pipeline: one search is one question (Fast) or several sub-questions
//! (Deep). Each question gives clean ranked lists (dense per workspace, Okapi BM25, file-name and
//! date matches); all lists of the search enter one Reciprocal Rank Fusion; one cross-encoder pass
//! rescores the best candidates, a few per document, up to a depth set by the mode, the hardware
//! profile and a time budget; a per-source cap and a character budget select the result.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use serde::{Deserialize, Serialize};

use crate::storage::lancedb::{NativeLanceStore, ScoredVectorResult};
use crate::ingestion::enricher::NativeContextualEnricher;
use crate::ingestion::embedding::QueryEmbedding;
use crate::retrieval::bm25::BM25Index;
use crate::retrieval::filter::RetrievalFilter;
use crate::retrieval::rerank_depth;
use crate::retrieval::rrf::{self, RankedList};
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
            max_chunks_per_source: self.max_chunks_per_source(),
            max_density_chars: self.max_density_chars(),
            table_name: "workspace_chunks".to_string(),
            rrf_k: self.rrf_k(),
            rerank_depth: rerank_depth::DEFAULT_RERANK_DEPTH,
            rerank_budget_ms: None,
            filter: RetrievalFilter::default(),
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
    pub max_chunks_per_source: usize,
    pub max_density_chars: usize,
    pub table_name: String,
    pub rrf_k: usize,
    /// Fused candidates the cross-encoder scores in this search, before the time budget (IMP-042).
    /// In a Deep batch, the first request's value applies.
    #[serde(default = "default_rerank_depth")]
    pub rerank_depth: usize,
    /// Time the cross-encoder may spend on this search; `None` keeps `rerank_depth` as it is.
    #[serde(default)]
    pub rerank_budget_ms: Option<u64>,
    /// Narrowing applied inside both retrievers, before their candidate limit (IMP-040).
    #[serde(default)]
    pub filter: RetrievalFilter,
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
            max_chunks_per_source: 3,
            max_density_chars: 40_000,
            table_name: "workspace_chunks".to_string(),
            rrf_k: 60,
            rerank_depth: rerank_depth::DEFAULT_RERANK_DEPTH,
            rerank_budget_ms: None,
            filter: RetrievalFilter::default(),
        }
    }
}

fn default_rerank_depth() -> usize {
    rerank_depth::DEFAULT_RERANK_DEPTH
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
        // Last, a fixed order so equal results always rank the same (IMP-065).
        .then_with(|| a.chunk_id.cmp(&b.chunk_id))
        .then_with(|| a.file_path.cmp(&b.file_path))
}

/// How the dense half of a hybrid search went for a query (IMP-003). A query without dense
/// search still answers from BM25 (ADR-110); this only makes that visible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryEmbeddingOutcome {
    /// The query was embedded (or a vector was supplied) and dense search ran.
    Dense,
    /// No embedding provider was configured for the pipeline.
    NoProvider,
    /// The workspace was indexed with the mock target (before ADR-0154, without an embedding
    /// API key): its vectors are all the same, so dense search is skipped until the next sync
    /// re-embeds it (BUG-019).
    NoEmbeddingKey,
    /// The workspace's embedding model cannot run (missing key, files or runtime): dense search
    /// is skipped, never done with another model (IMP-028).
    Unavailable(String),
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
            QueryEmbeddingOutcome::NoEmbeddingKey => "BM25 only (last query): indexed without an embedding model; sync to embed it".to_string(),
            QueryEmbeddingOutcome::Unavailable(reason) => format!("BM25 only (last query): {reason}"),
            QueryEmbeddingOutcome::Failed(reason) => format!("BM25 only (last query): query embedding failed: {reason}"),
            QueryEmbeddingOutcome::SearchFailed(reason) => format!("BM25 only (last query): dense search failed: {reason}"),
        }
    }
}

impl QueryEmbeddingOutcome {
    /// The warning line shown at the end of an answer whose search ran without the dense
    /// retriever, with the cause and the fix; `None` when dense search ran (IMP-040).
    pub fn answer_notice(&self) -> Option<String> {
        const LEAD: &str = "Answered by keyword search only";
        match self {
            QueryEmbeddingOutcome::Dense => None,
            QueryEmbeddingOutcome::NoProvider => Some(format!("{LEAD}: no embedding model is configured. Run `actx embedding` to choose one.")),
            QueryEmbeddingOutcome::NoEmbeddingKey => Some(format!("{LEAD}: this workspace was indexed without an embedding model. Sync it to embed it.")),
            QueryEmbeddingOutcome::Unavailable(reason) => Some(format!("{LEAD}: {}. Run `actx embedding` to check the model.", short_reason(reason))),
            QueryEmbeddingOutcome::Failed(reason) => Some(format!("{LEAD}: the question could not be embedded ({}).", short_reason(reason))),
            QueryEmbeddingOutcome::SearchFailed(reason) => Some(format!("{LEAD}: the semantic search failed ({}).", short_reason(reason))),
        }
    }
}

/// First line of an error, at most 160 characters, without a trailing period.
fn short_reason(reason: &str) -> String {
    let line = reason.lines().next().unwrap_or("").trim();
    let mut short: String = line.chars().take(160).collect();
    if line.chars().count() > 160 {
        short.push('…');
    }
    short.trim_end_matches('.').to_string()
}

/// How the reranking pass of a search went (IMP-043).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RerankOutcome {
    /// The cross-encoder scored the candidates; the unscored ones were discarded.
    Scored,
    /// Nothing to score: no candidate, or only pinned file-name matches.
    NotNeeded,
    /// The cross-encoder could not run: the fusion order was kept, nothing was discarded.
    Unavailable(String),
}

impl RerankOutcome {
    /// One-line description for status surfaces.
    pub fn describe(&self) -> String {
        match self {
            RerankOutcome::Scored => "ready (last query reranked)".to_string(),
            RerankOutcome::NotNeeded => "ready (last query had nothing to rerank)".to_string(),
            RerankOutcome::Unavailable(reason) => format!("unavailable (last query kept the fusion order): {reason}"),
        }
    }

    /// The warning line shown at the end of an answer whose candidates were not reranked, with
    /// the cause and the fix; `None` when the reranker ran or had nothing to do.
    pub fn answer_notice(&self) -> Option<String> {
        const LEAD: &str = "Answered without relevance ranking";
        let RerankOutcome::Unavailable(reason) = self else { return None };
        let lower = reason.to_lowercase();
        let fix = if lower.contains("weights") {
            "Run `actx download reranker` to install it."
        } else if lower.contains("runtime") {
            "Reinstall or update AnyContext to restore the ONNX Runtime library."
        } else {
            "Run `/status` to check the reranker."
        };
        Some(format!("{LEAD}: {}. {fix}", short_reason(reason)))
    }
}

/// Results of a search and how its dense half and its reranking went (IMP-040, IMP-043).
#[derive(Debug, Clone)]
pub struct RetrievalOutput {
    pub results: Vec<HybridSearchResult>,
    /// For a batch: the first sub-query whose dense half did not run, else `Dense`.
    pub dense: QueryEmbeddingOutcome,
    pub rerank: RerankOutcome,
}

impl RetrievalOutput {
    /// The warning lines for the end of the answer: dense search first, then the reranker.
    pub fn notices(&self) -> Option<String> {
        let lines: Vec<String> = [self.dense.answer_notice(), self.rerank.answer_notice()].into_iter().flatten().collect();
        (!lines.is_empty()).then(|| lines.join("\n"))
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

static LAST_RERANK_OUTCOME: std::sync::Mutex<Option<RerankOutcome>> = std::sync::Mutex::new(None);

/// Reranking outcome of the most recent hybrid search in this process, if any.
pub fn last_rerank_outcome() -> Option<RerankOutcome> {
    LAST_RERANK_OUTCOME.lock().ok().and_then(|o| o.clone())
}

/// Latency (ms) and candidate count of the most recent neural re-ranking pass, kept together so
/// concurrent passes never mix one's latency with another's count.
static LAST_RERANK_PASS: std::sync::Mutex<Option<(u64, usize)>> = std::sync::Mutex::new(None);

/// Wall-clock latency of the most recent neural re-ranking pass in this process, if any.
pub fn last_rerank_latency_ms() -> Option<u64> {
    last_rerank_pass().map(|(ms, _)| ms)
}

/// Candidates scored by the most recent neural re-ranking pass in this process (0 before any).
pub fn last_rerank_candidates() -> usize {
    last_rerank_pass().map_or(0, |(_, n)| n)
}

/// Latency (ms) and candidate count of the most recent neural re-ranking pass, if any.
pub fn last_rerank_pass() -> Option<(u64, usize)> {
    LAST_RERANK_PASS.lock().ok().and_then(|p| *p)
}

/// Why the cross-encoder cannot run, before any load is attempted: the runtime library is missing,
/// the weights are not installed, or an earlier load failed.
fn reranker_unavailable_reason(reranker: &actx_onnx::OnnxReranker) -> Option<String> {
    if !actx_onnx::runtime::runtime_library_present() {
        return Some("the ONNX Runtime library is missing".to_string());
    }
    if !reranker.weights_present() {
        return Some(format!("the reranker weights for `{}` are not installed", reranker.model_id()));
    }
    reranker.load_error().map(|e| format!("the reranker `{}` failed to load ({})", reranker.model_id(), short_reason(&e)))
}

/// Scores each group's texts against its question with the cross-encoder, in one pass on a
/// blocking thread so the async executor (and the TUI) never stall on CPU inference. An error
/// carries the cause and means "keep the fusion order". The model is loaded before the clock
/// starts, so the pass's cost per candidate, which feeds the time budget (IMP-042), never
/// includes the first load.
async fn neural_rerank_scores(
    reranker: &Arc<actx_onnx::OnnxReranker>,
    groups: Vec<(String, Vec<String>)>,
) -> Result<Vec<Vec<f32>>, String> {
    let total: usize = groups.iter().map(|(_, texts)| texts.len()).sum();
    if let Some(reason) = reranker_unavailable_reason(reranker) {
        return Err(reason);
    }
    let reranker = Arc::clone(reranker);
    let (scores, elapsed) = tokio::task::spawn_blocking(move || {
        reranker
            .ensure_loaded()
            .map_err(|e| format!("the reranker `{}` failed to load ({})", reranker.model_id(), short_reason(&e)))?;
        let started = std::time::Instant::now();
        let scores = groups
            .iter()
            .map(|(query, texts)| {
                let scores = reranker
                    .try_score_batch(query, texts)
                    .map_err(|e| format!("the reranker failed to score ({})", short_reason(&e)))?;
                if scores.len() != texts.len() {
                    return Err(format!("the reranker returned {} scores for {} candidates", scores.len(), texts.len()));
                }
                Ok(scores)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok::<_, String>((scores, started.elapsed()))
    })
    .await
    .map_err(|e| format!("the reranking task stopped ({e})"))??;
    if let Ok(mut last) = LAST_RERANK_PASS.lock() {
        *last = Some((elapsed.as_millis() as u64, total));
    }
    rerank_depth::record_rerank_cost(total, elapsed);
    Ok(scores)
}

/// Applies Stage-2 re-ranking to `picks` (chunk id → the question it is scored against): picked
/// items receive calibrated `[0,1]` scores and every other unpinned item is discarded (IMP-043).
/// `protected` ids (pinned file-name matches) keep their score and are never picked. When the
/// cross-encoder cannot run, the items keep the fusion order and nothing is discarded.
async fn apply_neural_rerank(
    reranker: Option<&Arc<actx_onnx::OnnxReranker>>,
    items: &mut Vec<HybridSearchResult>,
    picks: &HashMap<String, String>,
    protected: &std::collections::HashSet<String>,
) -> RerankOutcome {
    if picks.is_empty() {
        return RerankOutcome::NotNeeded;
    }
    let Some(reranker) = reranker else {
        return RerankOutcome::Unavailable("no reranker is configured".to_string());
    };
    // Picked items grouped by question, in item order, so each group is one cross-encoder batch.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let Some(query) = picks.get(&item.chunk_id) else { continue };
        if protected.contains(&item.chunk_id) {
            continue;
        }
        match groups.iter_mut().find(|(q, _)| q == query) {
            Some((_, idx)) => idx.push(i),
            None => groups.push((query.clone(), vec![i])),
        }
    }
    let batches = groups
        .iter()
        .map(|(q, idx)| (q.clone(), idx.iter().map(|&i| items[i].text.clone()).collect()))
        .collect();
    let scores = match neural_rerank_scores(reranker, batches).await {
        Ok(scores) => scores,
        Err(reason) => return RerankOutcome::Unavailable(reason),
    };
    for ((_, idx), group_scores) in groups.iter().zip(&scores) {
        for (&i, &s) in idx.iter().zip(group_scores) {
            items[i].score = s as f64;
        }
    }
    keep_scored(items, picks, protected);
    RerankOutcome::Scored
}

/// Keeps the pinned items and the scored picks; the candidates beyond the reranker depth are
/// discarded, never pushed to the end without a judgment (IMP-043).
fn keep_scored(items: &mut Vec<HybridSearchResult>, picks: &HashMap<String, String>, protected: &std::collections::HashSet<String>) {
    items.retain(|c| protected.contains(&c.chunk_id) || picks.contains_key(&c.chunk_id));
}

/// Reranker input: the unpinned `results` (in fusion order) through the per-document slots, up to
/// `depth`, each with the text of the question that ranked it best (IMP-041, IMP-042).
fn rerank_picks(
    results: &[HybridSearchResult],
    pinned: &std::collections::HashSet<String>,
    best_query: &HashMap<String, usize>,
    requests: &[HybridSearchRequest],
    depth: usize,
) -> HashMap<String, String> {
    let open: Vec<RankedChunk> = results.iter().filter(|c| !pinned.contains(&c.chunk_id)).map(ranked).collect();
    apply_source_cap(open, rerank_depth::PER_DOCUMENT_RERANK_SLOTS, depth)
        .into_iter()
        .map(|c| {
            let q = best_query.get(&c.id).copied().unwrap_or(0);
            (c.id, requests[q].query_text.clone())
        })
        .collect()
}

/// The fields the per-source cap and the character budget read.
fn ranked(c: &HybridSearchResult) -> RankedChunk {
    RankedChunk {
        id: c.chunk_id.clone(),
        score: c.score,
        file_path: c.file_path.clone(),
        file_name: c.file_name.clone(),
        text: c.text.clone(),
        workspace: c.workspace.clone(),
        content_type: c.content_type.clone(),
    }
}

/// What one question contributes to a search: its ranked lists and the data to build results.
#[derive(Default)]
struct QueryLists {
    lists: Vec<RankedList>,
    /// Stored rows of dense and metadata hits, by chunk id.
    rows: HashMap<String, ScoredVectorResult>,
    /// Best cosine similarity of each dense hit.
    dense_scores: HashMap<String, f32>,
    sparse_scores: HashMap<String, f32>,
    /// File-name matches: on top of the answer, never rescored.
    pinned: Vec<String>,
    dense: QueryEmbeddingOutcome,
}

impl Default for QueryEmbeddingOutcome {
    fn default() -> Self {
        QueryEmbeddingOutcome::Dense
    }
}

impl QueryLists {
    /// Keeps the stored row of a hit and returns its chunk id.
    fn keep_row(&mut self, row: ScoredVectorResult) -> String {
        let cid = if !row.id.is_empty() {
            row.id.clone()
        } else {
            format!("{}::{}", row.file_path, &row.text.chars().take(80).collect::<String>())
        };
        self.rows.entry(cid.clone()).or_insert(row);
        cid
    }

    fn push_list(&mut self, query: usize, ids: Vec<String>) {
        if !ids.is_empty() {
            self.lists.push(RankedList { query, ids });
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
    bm25_indices: Arc<RwLock<HashMap<String, Arc<BM25Index>>>>,
    lm_client: Option<Arc<dyn LmProvider>>,
    embedding_model: String,
    /// Prefix the embedding model expects in front of questions (IMP-029).
    query_prefix: String,
    /// Vector space of the question vectors: dense search only reads chunks of it (ADR-0154).
    query_target_key: Option<String>,
    reranker: Option<Arc<actx_onnx::OnnxReranker>>,
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
            query_prefix: String::new(),
            query_target_key: None,
            reranker: None,
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
            query_prefix: String::new(),
            query_target_key: None,
            reranker: None,
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
    /// used at ingestion time (see `ingestion::resolve_query_embedding`), otherwise query and
    /// chunk vectors live in different spaces.
    pub fn set_embedding(&mut self, lm_client: Option<Arc<dyn LmProvider>>, model: impl Into<String>) {
        self.lm_client = lm_client;
        self.embedding_model = model.into();
    }

    /// Sets the query embedding from `ingestion::resolve_query_embedding`: provider, model,
    /// query prefix and the vector space dense search is restricted to. Without a provider the
    /// outcome says why (mock index, or the model cannot run).
    pub fn set_query_embedding(&mut self, query: &QueryEmbedding) {
        self.set_embedding(query.provider.clone(), query.target.model.clone());
        self.query_prefix = query.query_prefix();
        self.query_target_key = (!query.target.is_mock() && query.target.dimension > 0).then(|| query.target.key());
        self.missing_provider_outcome = if query.target.is_mock() {
            QueryEmbeddingOutcome::NoEmbeddingKey
        } else if let Some(ref cause) = query.unavailable {
            QueryEmbeddingOutcome::Unavailable(cause.clone())
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


    /// Returns the database root directory path.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    /// Retrieves or loads the BM25 index for the given table. The index is shared, not copied,
    /// so a search costs no copy of it (IMP-040).
    pub fn get_or_load_bm25(&self, table_name: &str) -> Arc<BM25Index> {
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
        let index = writer.entry(table_name.to_string()).or_insert_with(|| Arc::new(index));
        Arc::clone(index)
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
            // Copies the index only while a search still holds the previous one.
            Arc::make_mut(idx).add_chunk(id, text, file_name, file_path, workspace, content_type, last_modified);
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
        writer.insert(table_name.to_string(), Arc::new(loaded));
        Ok(())
    }

    /// Executes single-query hybrid search asynchronously.
    pub async fn search_single_async(
        &self,
        req: &HybridSearchRequest,
    ) -> Result<Vec<HybridSearchResult>, String> {
        self.search_with_outcome(req).await.map(|out| out.results)
    }

    /// Single-query hybrid search that also says how its dense half went, so the answer can
    /// tell the user when it came from the lexical retriever only (IMP-040). It is a batch of
    /// one question: the same fusion, reranking and selection as Deep (IMP-041).
    pub async fn search_with_outcome(&self, req: &HybridSearchRequest) -> Result<RetrievalOutput, String> {
        let lists = self.collect_lists(req, 0).await?;
        Ok(self.fuse_and_rerank(std::slice::from_ref(req), vec![lists]).await)
    }

    /// Step 4 for one question: each retriever and each source gives its own list, ranked by its
    /// own score (IMP-041). Nothing here is scored for another list.
    async fn collect_lists(&self, req: &HybridSearchRequest, query: usize) -> Result<QueryLists, String> {
        // Workspaces searched: the request's own with the full list size, the others (`Global`)
        // with half of it.
        let mut targets = vec![(primary_workspace(req).to_string(), req.candidate_pool_k)];
        for tw in &req.target_workspaces {
            if !tw.is_empty() && !targets.iter().any(|(w, _)| w == tw) {
                targets.push((tw.clone(), std::cmp::max(10, req.candidate_pool_k / 2)));
            }
        }

        let mut out = QueryLists::default();
        let processed = QueryPreprocessor::process(&req.query_text);
        let filenames = processed.filename_mentions;
        let temporal_clauses = processed.temporal_clauses;
        let expanded_query = processed.expanded_query;

        // The request filter narrows every list inside its search, never after it (IMP-040).
        let filter_pred = req.filter.lance_predicate();
        let narrow = |clause: &str| match filter_pred {
            Some(ref f) => format!("({clause}) AND ({f})"),
            None => clause.to_string(),
        };

        // The lexical retriever starts now, on a blocking thread, and runs while the question is
        // embedded and the dense search runs (IMP-040).
        let lexical = {
            let index = self.get_or_load_bm25(&req.table_name);
            let query = expanded_query.clone();
            // `Default` is a workspace like any other: it sees its own files and `Global` (BUG-023).
            let workspace = primary_workspace(req).to_string();
            let filter = req.filter.clone();
            let limit = req.candidate_pool_k;
            tokio::task::spawn_blocking(move || index.search_filtered(&query, limit, Some(&workspace), &filter))
        };

        // File-name mentions: their chunks form one list and stay pinned on top of the answer.
        let mut filename_ids: Vec<String> = Vec::new();
        for fn_name in &filenames {
            let fn_clean = fn_name.replace('\'', "''");
            let fn_clause = format!("(file_name LIKE '%{fn_clean}%' OR file_path LIKE '%{fn_clean}%')");
            let combined_where = if temporal_clauses.is_empty() {
                narrow(&fn_clause)
            } else {
                narrow(&format!("({fn_clause}) AND ({})", temporal_clauses.join(" OR ")))
            };
            let fn_where = narrow(&fn_clause);

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
                        .search_metadata_async(&fn_where, *limit, ws_filter, Some(&req.table_name))
                        .await
                        .unwrap_or_default();
                }
                for row in matches {
                    let cid = out.keep_row(row);
                    if !filename_ids.contains(&cid) {
                        filename_ids.push(cid);
                    }
                }
            }
        }
        out.pinned = filename_ids.clone();
        out.push_list(query, filename_ids);

        // Date mentions: the chunks of the range form one list, in the order LanceDB returns them.
        if !temporal_clauses.is_empty() {
            let where_or = narrow(&temporal_clauses.join(" OR "));
            let mut date_ids: Vec<String> = Vec::new();
            for (ws_name, limit) in &targets {
                if let Ok(matches) = self
                    .lance_store
                    .search_metadata_async(&where_or, *limit, Some(ws_name.as_str()), Some(&req.table_name))
                    .await
                {
                    for row in matches {
                        let cid = out.keep_row(row);
                        if !date_ids.contains(&cid) {
                            date_ids.push(cid);
                        }
                    }
                }
            }
            out.push_list(query, date_ids);
        }

        // Query vector, passed in or computed. A failed or missing embedding leaves the lexical
        // list only (ADR-110); the outcome is recorded (IMP-003).
        let (query_vec, outcome) = if let Some(ref v) = req.query_vector {
            (Some(v.clone()), QueryEmbeddingOutcome::Dense)
        } else if let Some(ref lm) = self.lm_client {
            let question = format!("{}{}", self.query_prefix, req.query_text);
            match lm.embed(&self.embedding_model, &[question]).await {
                Ok(mut vecs) if !vecs.is_empty() => (Some(vecs.swap_remove(0)), QueryEmbeddingOutcome::Dense),
                Ok(_) => (None, QueryEmbeddingOutcome::Failed("empty embedding answer".to_string())),
                Err(e) => (None, QueryEmbeddingOutcome::Failed(e.to_string())),
            }
        } else {
            (None, self.missing_provider_outcome.clone())
        };
        self.record_query_embedding(outcome.clone());
        out.dense = outcome;

        // Dense search, one list per workspace. `Global` rows take part only when their vectors
        // share the query's space; otherwise the lexical and metadata lists still find them (BUG-019).
        let dense_filter = {
            let mut clauses = Vec::new();
            if !self.global_dense {
                clauses.push("workspace <> 'Global'".to_string());
            }
            if let Some(ref key) = self.query_target_key {
                clauses.push(NativeLanceStore::embedding_target_filter(key));
            }
            if let Some(ref f) = filter_pred {
                clauses.push(format!("({f})"));
            }
            (!clauses.is_empty()).then(|| clauses.join(" AND "))
        };
        if let Some(q_vec) = query_vec {
            let mut search_error = None;
            for (ws_name, limit) in &targets {
                if !self.global_dense && ws_name == "Global" {
                    continue;
                }
                // A failed search is recorded, never dropped silently (BUG-021).
                let mut matches = match self
                    .lance_store
                    .search_vector_async(q_vec.clone(), *limit, Some(ws_name.as_str()), dense_filter.as_deref(), Some(&req.table_name))
                    .await
                {
                    Ok(matches) => matches,
                    Err(e) => {
                        search_error.get_or_insert(e);
                        continue;
                    }
                };
                matches.sort_by(|a, b| {
                    b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal).then_with(|| a.id.cmp(&b.id))
                });
                let mut ids = Vec::with_capacity(matches.len());
                for row in matches {
                    let score = row.score;
                    let cid = out.keep_row(row);
                    let best = out.dense_scores.entry(cid.clone()).or_insert(score);
                    *best = best.max(score);
                    if !ids.contains(&cid) {
                        ids.push(cid);
                    }
                }
                out.push_list(query, ids);
            }
            if let Some(e) = search_error {
                out.dense = QueryEmbeddingOutcome::SearchFailed(e);
                self.record_query_embedding(out.dense.clone());
            }
        }

        // Lexical list, started above.
        let sparse = lexical.await.map_err(|e| format!("lexical search failed: {e}"))?;
        let ids = sparse.iter().map(|(id, _)| id.clone()).collect();
        out.sparse_scores = sparse.into_iter().collect();
        out.push_list(query, ids);
        Ok(out)
    }

    /// Steps 5 and 6 and today's selection for the lists of every question of one search: one
    /// RRF over all lists, one reranking pass over the chosen candidates, then the per-file cap,
    /// top-k and character budget of the first request (IMP-041, IMP-042).
    async fn fuse_and_rerank(&self, requests: &[HybridSearchRequest], per_query: Vec<QueryLists>) -> RetrievalOutput {
        let first = &requests[0];
        let label = |q: usize| {
            let r = &requests[q];
            r.sub_query_id.clone().unwrap_or_else(|| r.query_text.clone())
        };

        // Everything the questions found, merged; the dense outcome of the search is the first
        // question whose dense half did not run (IMP-040).
        let mut dense = QueryEmbeddingOutcome::Dense;
        let mut lists = Vec::new();
        let mut rows: HashMap<String, ScoredVectorResult> = HashMap::new();
        let mut dense_scores: HashMap<String, f32> = HashMap::new();
        let mut sparse_scores: HashMap<String, f32> = HashMap::new();
        let mut pinned: std::collections::HashSet<String> = std::collections::HashSet::new();
        for q in per_query {
            if dense == QueryEmbeddingOutcome::Dense {
                dense = q.dense;
            }
            lists.extend(q.lists);
            for (id, row) in q.rows {
                rows.entry(id).or_insert(row);
            }
            for (id, s) in q.dense_scores {
                let e = dense_scores.entry(id).or_insert(s);
                *e = e.max(s);
            }
            for (id, s) in q.sparse_scores {
                let e = sparse_scores.entry(id).or_insert(s);
                *e = e.max(s);
            }
            pinned.extend(q.pinned);
        }

        let fused = rrf::fuse(&lists, first.rrf_k);

        // Stored rows of lexical-only hits: the BM25 file holds the enveloped text it indexes and
        // no provenance, so text, content hash and provenance are read from LanceDB in one lookup
        // (ADR-0142, ADR-0143).
        let bm25_index = self.get_or_load_bm25(&first.table_name);
        let lexical_only: Vec<String> = fused.iter().filter(|f| !rows.contains_key(&f.id)).map(|f| f.id.clone()).collect();
        let mut stored_rows = self.lookup_stored_rows(&lexical_only, Some(primary_workspace(first)), &first.table_name).await;

        let mut results = Vec::with_capacity(fused.len());
        let mut best_query: HashMap<String, usize> = HashMap::new();
        for f in fused {
            let chunk_id = f.id;
            let (file_name, file_path, workspace, text, content_type, content_hash, last_modified, origin) =
                if let Some(d) = rows.remove(&chunk_id) {
                    let origin = Provenance::of(&d);
                    (
                        d.file_name,
                        d.file_path,
                        d.workspace,
                        d.text,
                        d.content_type.unwrap_or_else(|| "Local Document".to_string()),
                        d.content_hash.unwrap_or_default(),
                        d.last_modified,
                        origin,
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
                        doc.last_modified.clone(),
                        origin,
                    )
                } else {
                    continue;
                };

            let clear_text = if text.starts_with("enc::") {
                crate::security::NativeSecurityEngine::get_instance().decrypt_text(&text)
            } else {
                text
            };
            let score = if pinned.contains(&chunk_id) { 1.0 } else { f.score };
            best_query.insert(chunk_id.clone(), f.best_query);
            results.push(HybridSearchResult {
                token_count: estimate_token_count(&clear_text),
                dense_score: dense_scores.get(&chunk_id).copied(),
                sparse_score: sparse_scores.get(&chunk_id).copied(),
                chunk_id,
                file_name,
                file_path,
                workspace,
                text: clear_text,
                content_type,
                content_hash,
                score,
                matched_subqueries: f.queries.iter().map(|&q| label(q)).collect(),
                last_modified,
                header_path: origin.header_path,
                start_line: origin.start_line,
                end_line: origin.end_line,
            });
        }
        results.sort_by(compare_by_relevance);

        // Reranking: the unpinned candidates in fusion order, at most a few per document, up to
        // the depth of the mode and profile within its time budget (IMP-042). The cross-encoder
        // always runs; what it does not score is discarded (IMP-043).
        let depth = rerank_depth::effective_depth(first.rerank_depth, first.rerank_budget_ms);
        let picks = rerank_picks(&results, &pinned, &best_query, requests, depth);
        let rerank = apply_neural_rerank(self.reranker.as_ref(), &mut results, &picks, &pinned).await;
        if let Ok(mut last) = LAST_RERANK_OUTCOME.lock() {
            *last = Some(rerank.clone());
        }
        results.sort_by(compare_by_relevance);

        // Selection, once, with the first request's values (IMP-045 replaces it).
        let capped = apply_source_cap(results.iter().map(ranked).collect(), first.max_chunks_per_source, first.top_k);
        let budgeted = apply_density_budget(capped, first.max_density_chars);
        let mut by_id: HashMap<String, HybridSearchResult> = results.into_iter().map(|c| (c.chunk_id.clone(), c)).collect();
        let mut final_results = Vec::with_capacity(budgeted.len());
        for rc in budgeted {
            if let Some(mut full) = by_id.remove(&rc.id) {
                // The last chunk may have been condensed to fit the character budget.
                if full.text != rc.text {
                    full.token_count = estimate_token_count(&rc.text);
                    full.text = rc.text;
                }
                final_results.push(full);
            }
        }

        RetrievalOutput { results: final_results, dense, rerank }
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

    /// Deep Search batch: every sub-question collects its lists concurrently, then all lists enter
    /// one fusion and one reranking pass, and the selection runs once (IMP-041). A chunk found by
    /// several sub-questions is one candidate that lists all of them.
    pub async fn retrieve_hybrid_batch_async(
        &self,
        requests: &[HybridSearchRequest],
    ) -> Result<Vec<HybridSearchResult>, String> {
        self.retrieve_hybrid_batch_with_outcome(requests).await.map(|out| out.results)
    }

    /// [`Self::retrieve_hybrid_batch_async`] that also says how the dense half went: the first
    /// sub-query whose dense half did not run, or `Dense` when every one ran (IMP-040).
    pub async fn retrieve_hybrid_batch_with_outcome(
        &self,
        requests: &[HybridSearchRequest],
    ) -> Result<RetrievalOutput, String> {
        if requests.is_empty() {
            return Ok(RetrievalOutput { results: Vec::new(), dense: QueryEmbeddingOutcome::Dense, rerank: RerankOutcome::NotNeeded });
        }
        let futures: Vec<_> = requests.iter().enumerate().map(|(q, req)| self.collect_lists(req, q)).collect();
        let per_query = futures::future::join_all(futures).await.into_iter().collect::<Result<Vec<_>, _>>()?;
        Ok(self.fuse_and_rerank(requests, per_query).await)
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
    fn a_pipeline_without_a_neural_reranker_reports_why() {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_rerank_{}", std::process::id()));
        let mut pipeline = NativeHybridPipeline::open(&temp_dir).unwrap();
        assert!(pipeline.reranker().is_none());

        // Weights that do not exist: the reranker is not neural-active and says why.
        let dummy_reranker = actx_onnx::OnnxReranker::new(std::path::Path::new("missing_weights.onnx"));
        let reason = reranker_unavailable_reason(&dummy_reranker).expect("a cause");
        assert!(reason.contains("weights") || reason.contains("Runtime"), "{reason}");
        pipeline.set_reranker(Some(std::sync::Arc::new(dummy_reranker)));
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
        let outcome =
            apply_neural_rerank(Some(&reranker), &mut items, &picks(&["a", "b"], "what timeout"), &std::collections::HashSet::new()).await;
        assert!(matches!(outcome, RerankOutcome::Unavailable(_)), "{outcome:?}");
        assert_eq!(items.len(), 2, "nothing discarded");
        assert_eq!(items[0].chunk_id, "a");
        assert_eq!(items[0].score, 0.03);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_apply_neural_rerank_with_fixture_reorders_and_protects() {
        let Some(reranker) = fixture_reranker() else { return };
        let mut items = vec![
            result("theme", "the theme colors", 0.03),
            result("timeout", "timeout is thirty seconds", 0.02),
            result("pinned", "legacy theme", 0.95),
        ];
        let protected: std::collections::HashSet<String> = ["pinned".to_string()].into_iter().collect();
        let outcome = apply_neural_rerank(Some(&reranker), &mut items, &picks(&["theme", "timeout"], "what timeout"), &protected).await;
        assert_eq!(outcome, RerankOutcome::Scored);

        let score = |id: &str| items.iter().find(|c| c.chunk_id == id).unwrap().score;
        assert!(score("timeout") > score("theme"), "neural score must lift the timeout chunk");
        assert_eq!(score("pinned"), 0.95, "guaranteed filename matches keep their score");
        assert!(last_rerank_latency_ms().is_some());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_apply_neural_rerank_scores_only_the_picks_against_their_question() {
        let Some(reranker) = fixture_reranker() else { return };
        // Two pinned matches on top, picked candidates of two questions and one unpicked item.
        let mut items = vec![result("pin-a", "legacy theme", 1.0), result("pin-b", "legacy colors", 1.0)];
        let mut chosen = HashMap::new();
        for i in 0..6 {
            items.push(result(&format!("c{i}"), "timeout is thirty seconds", 0.5 - i as f64 * 0.001));
            let question = if i % 2 == 0 { "what timeout" } else { "theme colors" };
            chosen.insert(format!("c{i}"), question.to_string());
        }
        items.push(result("tail", "the theme colors", 0.4));
        let protected: std::collections::HashSet<String> =
            ["pin-a".to_string(), "pin-b".to_string()].into_iter().collect();
        assert_eq!(apply_neural_rerank(Some(&reranker), &mut items, &chosen, &protected).await, RerankOutcome::Scored);

        let score = |id: &str| items.iter().find(|c| c.chunk_id == id).unwrap().score;
        assert_eq!(score("pin-a"), 1.0);
        assert_eq!(score("pin-b"), 1.0);
        // Same text, two questions: each pick was scored against its own question.
        assert!(score("c0") > score("c1"), "c0 asked about timeouts, c1 about theme colors");
        assert_eq!(score("c0"), score("c2"));
        assert!(items.iter().all(|c| c.chunk_id != "tail"), "the unscored tail is discarded (IMP-043)");
    }

    /// The tiny cross-encoder of the ONNX fixtures, or `None` (test skipped) without ONNX Runtime.
    fn fixture_reranker() -> Option<std::sync::Arc<actx_onnx::OnnxReranker>> {
        if actx_onnx::runtime::probe_runtime().is_err() {
            if std::env::var("ACTX_REQUIRE_ORT").as_deref() == Ok("1") {
                panic!("ACTX_REQUIRE_ORT=1 but ONNX Runtime could not be loaded");
            }
            eprintln!("skipping: ONNX Runtime library unavailable (set ORT_DYLIB_PATH)");
            return None;
        }
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../actx-onnx/tests/fixtures");
        Some(std::sync::Arc::new(
            actx_onnx::OnnxReranker::with_paths(
                &fixtures.join("tiny_reranker.onnx"),
                &fixtures.join("tiny_tokenizer.json"),
                "tiny-fixture",
                actx_onnx::ExecutionDevice::Cpu,
            )
            .with_max_length(32),
        ))
    }

    fn picks(ids: &[&str], question: &str) -> HashMap<String, String> {
        ids.iter().map(|id| (id.to_string(), question.to_string())).collect()
    }

    /// SC-002: one document holding the 10 best fused chunks takes 3 of 8 slots while others have
    /// candidates; the reserve fills what is left, and pinned matches take no slot.
    #[test]
    fn rerank_picks_cap_each_document_and_fill_from_the_reserve() {
        let mut results: Vec<HybridSearchResult> = (0..10)
            .map(|i| HybridSearchResult { file_path: "docs/long.md".into(), ..result(&format!("long{i}"), "t", 1.0 - i as f64 * 0.01) })
            .collect();
        for i in 0..4 {
            results.push(result(&format!("short{i}"), "t", 0.5 - i as f64 * 0.01));
        }
        results.insert(0, result("pinned", "t", 1.0));
        let pinned: std::collections::HashSet<String> = ["pinned".to_string()].into_iter().collect();
        let requests = vec![HybridSearchRequest { query_text: "first".into(), ..Default::default() }];
        let chosen = rerank_picks(&results, &pinned, &HashMap::new(), &requests, 8);

        assert_eq!(chosen.len(), 8);
        assert!(!chosen.contains_key("pinned"));
        assert!((0..4).all(|i| chosen.contains_key(&format!("short{i}"))), "{chosen:?}");
        // 3 slots for the long document, plus 1 from its reserve in fusion order.
        let long: Vec<usize> = (0..10).filter(|i| chosen.contains_key(&format!("long{i}"))).collect();
        assert_eq!(long, vec![0, 1, 2, 3]);
    }

    #[test]
    fn rerank_picks_pair_each_candidate_with_its_best_question() {
        let results = vec![result("a", "t", 0.03), result("b", "t", 0.02)];
        let requests = vec![
            HybridSearchRequest { query_text: "first".into(), ..Default::default() },
            HybridSearchRequest { query_text: "second".into(), ..Default::default() },
        ];
        let best: HashMap<String, usize> = [("a".to_string(), 1), ("b".to_string(), 0)].into_iter().collect();
        let chosen = rerank_picks(&results, &Default::default(), &best, &requests, 30);
        assert_eq!(chosen["a"], "second");
        assert_eq!(chosen["b"], "first");
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
            embedding_target: None,
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
            embedding_target: None,
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
    fn compare_by_relevance_orders_full_ties_by_chunk_then_file() {
        let mut items = vec![result("b", "t", 0.5), result("a", "t", 0.5), HybridSearchResult { file_path: "docs/z.md".into(), ..result("a", "t", 0.5) }];
        items.sort_by(compare_by_relevance);
        let order: Vec<(&str, &str)> = items.iter().map(|c| (c.chunk_id.as_str(), c.file_path.as_str())).collect();
        assert_eq!(order, vec![("a", "docs/a.md"), ("a", "docs/z.md"), ("b", "docs/b.md")]);
    }

    #[test]
    fn no_candidate_is_dropped_by_its_rrf_score() {
        let chunks: Vec<(String, String, String)> = (0..30)
            .map(|i| (format!("h{i:02}"), format!("heron {}", "wading ".repeat(i + 1)), format!("h{i:02}.md")))
            .collect();
        let refs: Vec<(&str, &str, &str, &str)> =
            chunks.iter().map(|(id, text, file)| (id.as_str(), text.as_str(), file.as_str(), SAME_DAY)).collect();
        let (pipeline, dir) = bm25_pipeline("no_cut", &refs);
        let results = pipeline
            .retrieve_hybrid_batch(vec![HybridSearchRequest {
                query_text: "heron".to_string(),
                top_k: 100,
                candidate_pool_k: 30,
                max_density_chars: usize::MAX,
                ..Default::default()
            }])
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results.len(), 30, "every candidate of a single list survives");
        assert_eq!(results.last().unwrap().score, 1.0 / 90.0);
    }

    /// IMP-041: Deep fuses the lists of all sub-questions once; a chunk found by both scores the
    /// sum of its RRF contributions (not of two finished results), and one found by one only keeps
    /// its single contribution.
    #[test]
    fn a_batch_fuses_the_lists_of_every_sub_question_once() {
        let (pipeline, dir) = bm25_pipeline(
            "one_fusion",
            &[("both", "falcon kestrel", "both.md", SAME_DAY), ("one", "falcon falcon falcon dive", "one.md", SAME_DAY)],
        );
        let sub = |id: &str, q: &str| HybridSearchRequest { sub_query_id: Some(id.into()), query_text: q.into(), ..Default::default() };
        let results = pipeline.retrieve_hybrid_batch(vec![sub("q1", "falcon"), sub("q2", "kestrel")]).unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let both = results.iter().find(|r| r.chunk_id == "both").unwrap();
        let one = results.iter().find(|r| r.chunk_id == "one").unwrap();
        assert_eq!(both.matched_subqueries, vec!["q1".to_string(), "q2".to_string()]);
        assert_eq!(one.matched_subqueries, vec!["q1".to_string()]);
        // "both" is second for "falcon" and first for "kestrel".
        assert!((both.score - (1.0 / 62.0 + 1.0 / 61.0)).abs() < 1e-12);
        assert!((one.score - 1.0 / 61.0).abs() < 1e-12);
        assert_eq!(results[0].chunk_id, "both");
    }

    /// IMP-041: a date mention gives its own list, ranked by LanceDB's order, with no invented
    /// score: the chunk of the date range collects its date and dense contributions, nothing more.
    #[test]
    fn date_matches_are_their_own_list_without_an_invented_score() {
        let (pipeline, dir) = bm25_pipeline("date_list", &[]);
        let rows = vec![
            stored_row("d_0", "the incident report", "2026-09-02-report.md", vec![0.0, 1.0, 0.0, 0.0]),
            stored_row("x_0", "an unrelated note", "other.md", vec![1.0, 0.0, 0.0, 0.0]),
        ];
        pipeline.lance_store.upsert_records(rows, None, Some(4)).unwrap();
        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "what happened on 2026-09-02".to_string(),
                query_vector: Some(vec![1.0, 0.0, 0.0, 0.0]),
                ..Default::default()
            })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let score = |id: &str| results.iter().find(|r| r.chunk_id == id).unwrap().score;
        // d_0: first of the date list, second of the dense list; x_0: first of the dense list.
        assert!((score("d_0") - (1.0 / 61.0 + 1.0 / 62.0)).abs() < 1e-12, "{results:?}");
        assert!((score("x_0") - 1.0 / 61.0).abs() < 1e-12, "{results:?}");
    }

    /// IMP-041: `Global`'s dense hits are a list of their own, so a chunk that `Default`'s dense
    /// search also sees (BUG-023) collects one contribution from each list.
    #[test]
    fn global_dense_hits_are_their_own_list() {
        let (mut pipeline, dir) = bm25_pipeline("global_list", &[]);
        let own = stored_row("o_0", "own notes", "own.md", vec![1.0, 0.0, 0.0, 0.0]);
        let global = crate::storage::VectorRecord {
            workspace: "Global".into(),
            file_path: "system://README.md".into(),
            ..stored_row("g_0", "AnyContext guide", "README.md", vec![1.0, 0.0, 0.0, 0.0])
        };
        pipeline.lance_store.upsert_records(vec![own, global], None, Some(4)).unwrap();
        pipeline.set_global_dense(true);
        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "zzz".to_string(),
                query_vector: Some(vec![1.0, 0.0, 0.0, 0.0]),
                target_workspaces: vec!["Global".to_string()],
                ..Default::default()
            })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let score = |id: &str| results.iter().find(|r| r.chunk_id == id).unwrap().score;
        // Equal similarities rank by chunk id: g_0 is first in `Default`'s list and in `Global`'s.
        assert!((score("g_0") - 2.0 / 61.0).abs() < 1e-12, "{results:?}");
        assert!((score("o_0") - 1.0 / 62.0).abs() < 1e-12, "{results:?}");
    }

    /// US1: in a Deep batch each candidate gets one cross-encoder score, against the
    /// sub-question that ranked it best; nothing is summed across sub-questions.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_batch_reranks_each_candidate_once_against_its_best_question() {
        let Some(reranker) = fixture_reranker() else { return };
        let (mut pipeline, dir) = bm25_pipeline(
            "batch_rerank",
            &[("both", "falcon kestrel", "both.md", SAME_DAY), ("one", "falcon falcon falcon dive", "one.md", SAME_DAY)],
        );
        pipeline.set_reranker(Some(reranker.clone()));
        let sub = |id: &str, q: &str| HybridSearchRequest { sub_query_id: Some(id.into()), query_text: q.into(), ..Default::default() };
        let results = pipeline.retrieve_hybrid_batch_async(&[sub("q1", "falcon"), sub("q2", "kestrel")]).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);

        let expected = |q: &str, text: &str| reranker.score_batch(q, &[text.to_string()])[0] as f64;
        let score = |id: &str| results.iter().find(|r| r.chunk_id == id).unwrap().score;
        // "both" is first for "kestrel" and second for "falcon"; "one" is first for "falcon".
        assert!((score("both") - expected("kestrel", "falcon kestrel")).abs() < 1e-4);
        assert!((score("one") - expected("falcon", "falcon falcon falcon dive")).abs() < 1e-4);
        assert_eq!(results.iter().find(|r| r.chunk_id == "both").unwrap().matched_subqueries, vec!["q1", "q2"]);
    }

    /// IMP-043: with more candidates than the depth, only the scored ones (and pinned matches) go
    /// on; the rest is discarded instead of being kept below them.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn candidates_beyond_the_depth_are_discarded() {
        let Some(reranker) = fixture_reranker() else { return };
        let chunks: Vec<(String, String, String)> =
            (0..5).map(|i| (format!("c_{i}"), format!("falcon note {i}"), format!("f{i}.md"))).collect();
        let refs: Vec<(&str, &str, &str, &str)> = chunks.iter().map(|(id, t, f)| (id.as_str(), t.as_str(), f.as_str(), SAME_DAY)).collect();
        let (mut pipeline, dir) = bm25_pipeline("tail_discarded", &refs);
        pipeline.set_reranker(Some(reranker));
        let req = HybridSearchRequest { query_text: "falcon".into(), rerank_depth: 2, ..Default::default() };
        let output = pipeline.search_with_outcome(&req).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(output.rerank, RerankOutcome::Scored);
        assert_eq!(output.results.len(), 2, "{:?}", output.results);
        assert!(output.results.windows(2).all(|w| w[0].score >= w[1].score));
        assert!(output.rerank.answer_notice().is_none());
    }

    /// IMP-043: fewer candidates than the depth are all scored and kept.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn candidates_within_the_depth_are_all_kept() {
        let Some(reranker) = fixture_reranker() else { return };
        let (mut pipeline, dir) = bm25_pipeline(
            "within_depth",
            &[("a", "falcon dive", "a.md", SAME_DAY), ("b", "falcon nest", "b.md", SAME_DAY)],
        );
        pipeline.set_reranker(Some(reranker));
        let output = pipeline.search_with_outcome(&HybridSearchRequest { query_text: "falcon".into(), ..Default::default() }).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(output.rerank, RerankOutcome::Scored);
        assert_eq!(output.results.len(), 2);
    }

    /// IMP-043: a reranker that cannot run keeps the fusion order, discards nothing and says why.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn an_unavailable_reranker_keeps_the_fusion_order_and_reports_it() {
        let chunks: Vec<(String, String, String)> =
            (0..5).map(|i| (format!("c_{i}"), format!("falcon note {i}"), format!("f{i}.md"))).collect();
        let refs: Vec<(&str, &str, &str, &str)> = chunks.iter().map(|(id, t, f)| (id.as_str(), t.as_str(), f.as_str(), SAME_DAY)).collect();
        let req = HybridSearchRequest { query_text: "falcon".into(), rerank_depth: 2, ..Default::default() };

        // No reranker at all.
        let (pipeline, dir) = bm25_pipeline("no_reranker", &refs);
        let output = pipeline.search_with_outcome(&req).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);
        assert_eq!(output.rerank, RerankOutcome::Unavailable("no reranker is configured".into()));
        assert_eq!(output.results.len(), 5, "nothing discarded");
        let notices = output.notices().unwrap();
        assert!(notices.lines().last().unwrap().starts_with("Answered without relevance ranking: no reranker is configured."), "{notices}");

        // A reranker whose weights are missing.
        let (mut pipeline, dir) = bm25_pipeline("missing_weights", &refs);
        pipeline.set_reranker(Some(Arc::new(actx_onnx::OnnxReranker::new(std::path::Path::new("missing_weights.onnx")))));
        let output = pipeline.search_with_outcome(&req).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);
        assert!(matches!(output.rerank, RerankOutcome::Unavailable(_)), "{:?}", output.rerank);
        assert_eq!(output.results.len(), 5);
    }

    /// IMP-043: only pinned file-name matches means nothing to rerank, and no warning.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn only_pinned_matches_need_no_reranking() {
        let (pipeline, dir) = bm25_pipeline("only_pinned", &[]);
        pipeline
            .lance_store
            .upsert_records(vec![stored_row("g_0", "the guide intro", "guide.md", vec![0.0, 1.0, 0.0, 0.0])], None, Some(4))
            .unwrap();
        let output = pipeline.search_with_outcome(&HybridSearchRequest { query_text: "guide.md".into(), ..Default::default() }).await.unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(output.results.iter().map(|r| r.chunk_id.as_str()).collect::<Vec<_>>(), vec!["g_0"]);
        assert_eq!(output.rerank, RerankOutcome::NotNeeded);
        assert!(output.rerank.answer_notice().is_none());
    }

    #[test]
    fn keep_scored_keeps_pinned_and_picked_only() {
        let mut items = vec![result("p", "pinned", 1.0), result("a", "a", 0.9), result("b", "b", 0.5), result("c", "c", 0.1)];
        let picks: HashMap<String, String> = [("a", "q"), ("c", "q")].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        let pinned: std::collections::HashSet<String> = ["p".to_string()].into_iter().collect();
        keep_scored(&mut items, &picks, &pinned);
        assert_eq!(items.iter().map(|c| c.chunk_id.as_str()).collect::<Vec<_>>(), vec!["p", "a", "c"]);
    }

    #[test]
    fn rerank_notices_name_the_cause_and_the_fix() {
        assert_eq!(RerankOutcome::Scored.answer_notice(), None);
        assert_eq!(RerankOutcome::NotNeeded.answer_notice(), None);
        let weights = RerankOutcome::Unavailable("the reranker weights for `mmarco-reranker` are not installed".into());
        assert_eq!(
            weights.answer_notice().unwrap(),
            "Answered without relevance ranking: the reranker weights for `mmarco-reranker` are not installed. Run `actx download reranker` to install it."
        );
        let runtime = RerankOutcome::Unavailable("the ONNX Runtime library is missing".into());
        assert!(runtime.answer_notice().unwrap().ends_with("Reinstall or update AnyContext to restore the ONNX Runtime library."));
        let failed = RerankOutcome::Unavailable("the reranker failed to score (bad input)".into());
        assert!(failed.answer_notice().unwrap().ends_with("Run `/status` to check the reranker."));
    }

    #[test]
    fn notices_list_dense_then_reranker() {
        let output = RetrievalOutput {
            results: Vec::new(),
            dense: QueryEmbeddingOutcome::NoProvider,
            rerank: RerankOutcome::Unavailable("the ONNX Runtime library is missing".into()),
        };
        let notices = output.notices().unwrap();
        let lines: Vec<&str> = notices.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("Answered by keyword search only"));
        assert!(lines[1].starts_with("Answered without relevance ranking"));
        let clean = RetrievalOutput { results: Vec::new(), dense: QueryEmbeddingOutcome::Dense, rerank: RerankOutcome::Scored };
        assert_eq!(clean.notices(), None);
    }

    /// File-name matches form their own list and stay pinned on top; their dense score is the
    /// real similarity, not an invented 1.0 (IMP-041).
    #[test]
    fn file_name_matches_are_their_own_list_and_stay_pinned() {
        let (pipeline, dir) = bm25_pipeline("filename_list", &[("b_0", "kestrel kestrel kestrel guide", "beta.md", SAME_DAY)]);
        let rows = vec![
            stored_row("g_0", "the guide intro", "guide.md", vec![0.0, 1.0, 0.0, 0.0]),
            stored_row("b_0", "kestrel kestrel kestrel guide", "beta.md", vec![1.0, 0.0, 0.0, 0.0]),
        ];
        pipeline.lance_store.upsert_records(rows, None, Some(4)).unwrap();
        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "kestrel guide.md".to_string(),
                query_vector: Some(vec![1.0, 0.0, 0.0, 0.0]),
                ..Default::default()
            })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(results[0].chunk_id, "g_0", "{results:?}");
        assert_eq!(results[0].score, 1.0);
        let g = &results[0];
        assert!(g.dense_score.map_or(true, |d| d < 0.5), "no invented dense score: {:?}", g.dense_score);
        let b = results.iter().find(|r| r.chunk_id == "b_0").unwrap();
        assert!(b.score < 1.0);
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
            "BM25 only (last query): indexed without an embedding model; sync to embed it"
        );
        assert_eq!(
            QueryEmbeddingOutcome::Unavailable("Embedding model bge-m3 is unavailable: offline. Fix.".into()).describe(),
            "BM25 only (last query): Embedding model bge-m3 is unavailable: offline. Fix."
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
        // A workspace indexed with the mock target before ADR-0154.
        let db = crate::storage::NativeConfigDb::open_in_memory().expect("db");
        db.set_workspace_embedding_target("Default", Some("mock/mock-embed")).expect("record");
        let query = crate::ingestion::resolve_query_embedding(&db, "Default");
        assert!(query.target.is_mock() && query.provider.is_none(), "{query:?}");

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
        pipeline.set_query_embedding(&query);

        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "kestrel timeouts".to_string(),
                top_k: 8,
                candidate_pool_k: 30,
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
    /// Records the question it embeds and answers with a unit vector of the given dimension.
    struct RecordingQueryEmbedder {
        dimension: usize,
        texts: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait::async_trait]
    impl LmProvider for RecordingQueryEmbedder {
        fn provider_id(&self) -> &'static str {
            "recording"
        }

        async fn chat_complete(&self, _r: actx_lm::types::ChatRequest) -> Result<actx_lm::types::ChatResponse, actx_lm::error::LmError> {
            Err(actx_lm::error::LmError::Unsupported("chat".into()))
        }

        async fn chat_stream(&self, _r: actx_lm::types::ChatRequest) -> Result<actx_lm::traits::BoxedChunkStream, actx_lm::error::LmError> {
            Err(actx_lm::error::LmError::Unsupported("chat".into()))
        }

        async fn embed(&self, _model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, actx_lm::error::LmError> {
            self.texts.lock().unwrap().extend(texts.iter().cloned());
            let mut v = vec![0.0; self.dimension];
            v[0] = 1.0;
            Ok(vec![v; texts.len()])
        }
    }

    /// IMP-029 and ADR-0154: the question reaches the model with the query prefix of the
    /// workspace's space, and dense search only returns chunks of that space.
    #[test]
    fn test_query_prefix_and_space_filter() {
        let spec = crate::ingestion::find_embedding_model("multilingual-e5-small").unwrap();
        let recorder = Arc::new(RecordingQueryEmbedder { dimension: spec.dimension, texts: Default::default() });
        let query = QueryEmbedding { provider: Some(recorder.clone()), target: spec.target(), spec: Some(spec), unavailable: None };

        let (mut pipeline, dir) = bm25_pipeline("query_prefix", &[]);
        let dim = crate::storage::lancedb::DEFAULT_VECTOR_DIM;
        let mut unit = vec![0.0; dim];
        unit[0] = 1.0;
        let mut same_space = stored_row("e5_0", "zebra migration notes", "zebra.md", unit.clone());
        same_space.embedding_target = Some(spec.target().key());
        let mut other_space = stored_row("oa_0", "walrus feeding schedule", "walrus.md", unit);
        other_space.embedding_target = Some("openai/text-embedding-3-small@1536".into());
        pipeline.lance_store.upsert_records(vec![same_space, other_space], None, Some(dim)).unwrap();
        pipeline.set_query_embedding(&query);

        let results = pipeline
            .search(HybridSearchRequest {
                query_text: "onde ficam as notas".to_string(),
                top_k: 8,
                candidate_pool_k: 30,
                ..Default::default()
            })
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert_eq!(recorder.texts.lock().unwrap().as_slice(), ["query: onde ficam as notas"]);
        let dense: Vec<&str> = results.iter().filter(|r| r.dense_score.is_some()).map(|r| r.chunk_id.as_str()).collect();
        assert_eq!(dense, vec!["e5_0"], "a chunk of another vector space came from dense search");
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

    /// Workspace of a large source with strong matches and a small one with weak matches, in
    /// both LanceDB and BM25 (IMP-040).
    fn two_source_pipeline(name: &str) -> (NativeHybridPipeline, PathBuf) {
        let (pipeline, dir) = bm25_pipeline(name, &[]);
        let mut rows = Vec::new();
        let mut add = |id: String, text: &str, path: &str, modified: &str, content_type: &str, vector: Vec<f32>| {
            let mut row = stored_row(&id, text, path.rsplit('/').next().unwrap(), vector);
            row.file_path = path.to_string();
            row.last_modified = Some(modified.to_string());
            row.content_type = Some(content_type.to_string());
            pipeline.add_chunk_to_bm25(
                "workspace_chunks", id, text.to_string(), row.file_name.clone(), path.to_string(),
                "Default".to_string(), content_type.to_string(), Some(modified.to_string()),
            );
            rows.push(row);
        };
        for i in 0..40 {
            add(format!("big_{i:02}"), "invoice invoice invoice payment terms", &format!("/data/big/b{i:02}.md"),
                "2025-01-01T00:00:00+00:00", "markdown", vec![1.0, 0.01 * i as f32, 0.0, 0.0]);
        }
        for i in 0..3 {
            add(format!("small_{i}"), "a note that mentions an invoice once among warehouse staff matters",
                &format!("/data/small/s{i}.md"), "2025-06-01T00:00:00+00:00", "code", vec![0.0, 0.0, 1.0, 0.1 * i as f32]);
        }
        pipeline.lance_store.upsert_records(rows, None, Some(4)).unwrap();
        (pipeline, dir)
    }

    fn narrowed(filter: RetrievalFilter) -> HybridSearchRequest {
        HybridSearchRequest {
            query_text: "invoice".to_string(),
            query_vector: Some(vec![1.0, 0.0, 0.0, 0.0]),
            candidate_pool_k: 30,
            top_k: 100,
            max_chunks_per_source: 100,
            max_density_chars: 1_000_000,
            filter,
            ..Default::default()
        }
    }

    /// SC-001: without the filter the small source is outranked in both lists; with it, both
    /// lists hold exactly the small source's chunks.
    #[test]
    fn test_source_filter_applies_inside_both_retrievers() {
        let (pipeline, dir) = two_source_pipeline("filter_sources");
        let unfiltered = pipeline.search(narrowed(RetrievalFilter::default())).unwrap();
        let small = RetrievalFilter { sources: vec!["/data/small".to_string()], ..Default::default() };
        let filtered = pipeline.search(narrowed(small)).unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert!(unfiltered.iter().all(|r| !r.chunk_id.starts_with("small_")), "the large source outranks the small one");
        let mut ids: Vec<&str> = filtered.iter().map(|r| r.chunk_id.as_str()).collect();
        ids.sort();
        assert_eq!(ids, vec!["small_0", "small_1", "small_2"]);
        assert!(filtered.iter().all(|r| r.dense_score.is_some() && r.sparse_score.is_some()), "both lists found them");
    }

    #[test]
    fn test_date_and_type_filters_apply_inside_both_retrievers() {
        let (pipeline, dir) = two_source_pipeline("filter_dates");
        let june = RetrievalFilter::default().with_modified_range(
            Some(chrono::DateTime::parse_from_rfc3339("2025-05-01T00:00:00+00:00").unwrap().with_timezone(&chrono::Utc)),
            None,
        );
        let by_date = pipeline.search(narrowed(june)).unwrap();
        let by_type = pipeline.search(narrowed(RetrievalFilter { content_types: vec!["code".to_string()], ..Default::default() })).unwrap();
        let _ = std::fs::remove_dir_all(dir);

        for results in [&by_date, &by_type] {
            assert_eq!(results.len(), 3, "{results:?}");
            assert!(results.iter().all(|r| r.chunk_id.starts_with("small_") && r.dense_score.is_some()));
        }
    }

    #[test]
    fn test_filter_matching_nothing_returns_nothing() {
        let (pipeline, dir) = two_source_pipeline("filter_nothing");
        let none = RetrievalFilter { sources: vec!["/data/elsewhere".to_string()], ..Default::default() };
        let quoted = RetrievalFilter { sources: vec!["/data/small' OR '1'='1".to_string()], ..Default::default() };
        let wildcard = RetrievalFilter { sources: vec!["/data/%".to_string()], ..Default::default() };
        let results: Vec<_> = [none, quoted, wildcard].into_iter().map(|f| pipeline.search(narrowed(f)).unwrap()).collect();
        let _ = std::fs::remove_dir_all(dir);
        for r in results {
            assert!(r.is_empty(), "{r:?}");
        }
    }

    /// The lexical list answers when the question cannot be embedded, and the search says why.
    #[test]
    fn test_search_with_outcome_reports_keyword_only_search() {
        let (mut pipeline, dir) = two_source_pipeline("outcome_failed");
        pipeline.set_embedding(Some(Arc::new(FixedEmbedder(Err(()))) as Arc<dyn LmProvider>), "fixed-embed");
        let req = HybridSearchRequest { query_vector: None, ..narrowed(RetrievalFilter::default()) };
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        let single = rt.block_on(pipeline.search_with_outcome(&req)).unwrap();
        let batch = rt.block_on(pipeline.retrieve_hybrid_batch_with_outcome(&[req.clone(), req.clone()])).unwrap();
        let _ = std::fs::remove_dir_all(dir);

        assert!(!single.results.is_empty(), "BM25 still answers");
        assert!(single.results.iter().all(|r| r.dense_score.is_none()));
        assert!(matches!(single.dense, QueryEmbeddingOutcome::Failed(_)), "{:?}", single.dense);
        let notice = single.dense.answer_notice().unwrap();
        assert!(notice.starts_with("Answered by keyword search only"), "{notice}");
        assert!(notice.contains("invalid key"), "{notice}");
        assert!(matches!(batch.dense, QueryEmbeddingOutcome::Failed(_)));
    }

    #[test]
    fn test_search_with_outcome_is_dense_when_the_vector_search_ran() {
        let (pipeline, dir) = two_source_pipeline("outcome_dense");
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        let out = rt.block_on(pipeline.search_with_outcome(&narrowed(RetrievalFilter::default()))).unwrap();
        let _ = std::fs::remove_dir_all(dir);
        assert_eq!(out.dense, QueryEmbeddingOutcome::Dense);
        assert_eq!(out.dense.answer_notice(), None);
    }

    #[test]
    fn test_answer_notice_names_cause_and_fix() {
        assert_eq!(QueryEmbeddingOutcome::Dense.answer_notice(), None);
        let unavailable = QueryEmbeddingOutcome::Unavailable("the model multilingual-e5-base is not installed".into()).answer_notice().unwrap();
        assert_eq!(unavailable, "Answered by keyword search only: the model multilingual-e5-base is not installed. Run `actx embedding` to check the model.");
        let long = QueryEmbeddingOutcome::Failed(format!("{}\nsecond line", "x".repeat(300))).answer_notice().unwrap();
        assert!(!long.contains("second line") && long.contains('…'), "{long}");
        assert!(QueryEmbeddingOutcome::NoEmbeddingKey.answer_notice().unwrap().contains("Sync it"));
        assert!(QueryEmbeddingOutcome::SearchFailed("io".into()).answer_notice().unwrap().contains("semantic search failed (io)"));
    }

}
