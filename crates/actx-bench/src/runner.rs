//! Runs the question file through the product's ingestion and retrieval in an isolated sandbox.
//!
//! Every path the core may open by default (settings, vector store, models, logs) is pointed at
//! the sandbox before the core is touched, so a run never reads or writes the developer's own
//! workspaces, settings or keys.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use any_context_core_rs::ingestion::{
    find_embedding_model, resolve_query_embedding, EmbeddingModelSpec, EmbeddingTarget, NativeSyncOrchestrator, OnnxModelManager,
    SyncOptions, DEFAULT_RERANKER_ID,
};
use any_context_core_rs::hardware::ProfileTier;
use any_context_core_rs::retrieval::{fast_request, HybridSearchRequest, HybridSearchResult, NativeHybridPipeline, RerankOutcome};
use any_context_core_rs::storage::{NativeConfigDb, NativeLanceStore};

use crate::corpus::{hash_bytes, normalize_whitespace, Corpus};
use crate::metrics::{self, Outcome, RankedDoc, DEPTH};
use crate::questions::{validate, LabelError, Question, QuestionFile};
use crate::result::{Retrievers, RunConfig, RunResult, SCHEMA};

const WORKSPACE: &str = "Benchmark";
const TABLE: &str = "workspace_chunks";

/// Why a run could not produce a result.
#[derive(Debug)]
pub enum RunError {
    /// Labels point to documents or passages that are not there.
    Labels(Vec<LabelError>),
    /// The models, the provider, the files or the index are not usable.
    Environment(String),
}

impl From<String> for RunError {
    fn from(e: String) -> Self {
        RunError::Environment(e)
    }
}

pub struct RunOptions {
    pub corpus_dir: PathBuf,
    pub questions_file: PathBuf,
    /// Embedding target (`provider/model`) for a dense run; `None` keeps the mock target (no dense).
    pub embedding: Option<String>,
    /// Where the reranker is cached; the sandbox gets its own folder when `None`.
    pub models_dir: Option<PathBuf>,
    pub check_determinism: bool,
}

/// Sandbox folder of one run, removed when the run ends.
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    /// Creates the sandbox and points the core's default paths at it.
    pub fn enter(models_dir: Option<&Path>) -> Result<Self, String> {
        let root = std::env::temp_dir().join(format!("actx_bench_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("config")).map_err(|e| format!("sandbox cannot be created: {e}"))?;
        let models = models_dir.map(Path::to_path_buf).unwrap_or_else(|| root.join("models"));
        std::env::set_var("ACTX_TEST_MODE", "1");
        std::env::set_var("ACTX_SETTINGS_DB", root.join("config").join("settings.db"));
        std::env::set_var("ACTX_CONTEXT_DB", root.join("context_db").join("lancedb"));
        std::env::set_var("ACTX_MODELS_DIR", &models);
        Ok(Self { root })
    }

    pub fn settings_db(&self) -> PathBuf {
        self.root.join("config").join("settings.db")
    }

    pub fn lancedb(&self) -> PathBuf {
        self.root.join("context_db").join("lancedb")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Loads the corpus and the questions and checks the labels that can be checked without an index.
pub fn load_and_validate(corpus_dir: &Path, questions_file: &Path) -> Result<(Corpus, QuestionFile, String), RunError> {
    let corpus = Corpus::load(corpus_dir)?;
    let bytes = std::fs::read(questions_file)
        .map_err(|e| format!("'{}' cannot be read: {e}", questions_file.display()))?;
    let text = String::from_utf8(bytes.clone()).map_err(|e| format!("'{}' is not UTF-8: {e}", questions_file.display()))?;
    let file = QuestionFile::parse(&text).map_err(|e| RunError::Labels(vec![LabelError { question: "-".into(), reason: e }]))?;
    let errors = validate(&file, &corpus);
    if !errors.is_empty() {
        return Err(RunError::Labels(errors));
    }
    Ok((corpus, file, hash_bytes(&bytes)))
}

pub async fn run(options: &RunOptions) -> Result<RunResult, RunError> {
    let (corpus, file, questions_version) = load_and_validate(&options.corpus_dir, &options.questions_file)?;
    let sandbox = Sandbox::enter(options.models_dir.as_deref())?;

    let db = Arc::new(NativeConfigDb::open(sandbox.settings_db()).map_err(|e| format!("settings: {e}"))?);
    let lance = Arc::new(NativeLanceStore::open(sandbox.lancedb())?);
    db.create_workspace(WORKSPACE, Some("Retrieval benchmark")).map_err(|e| format!("workspace: {e}"))?;
    db.add_workspace_folder(WORKSPACE, &corpus.root().to_string_lossy()).map_err(|e| format!("workspace folder: {e}"))?;

    let spec = expected_model(options.embedding.as_deref())?;
    let target = spec.target();
    index(&db, &lance, &corpus, spec).await?;
    check_indexed_passages(&lance, &corpus, &file).await?;

    let reranker = load_reranker().await?;
    let mut pipeline = NativeHybridPipeline::new(lance.clone(), None);
    // The question side resolves exactly as the product does, from the recorded target.
    let query = resolve_query_embedding(&db, WORKSPACE);
    if let Some(ref cause) = query.unavailable {
        return Err(RunError::Environment(format!("query embedding unavailable: {cause}")));
    }
    pipeline.set_query_embedding(&query);
    pipeline.set_reranker(Some(reranker));

    // Warm-up: the first query pays for lazy loading (BM25 file, ONNX session); it is not timed.
    if let Some(first) = file.questions.first() {
        let _ = pipeline.search_single_async(&delivered_request(&first.text)).await;
    }

    let mut outcomes = Vec::with_capacity(file.questions.len());
    for q in &file.questions {
        outcomes.push(ask(&pipeline, &corpus, q).await?);
    }
    if options.check_determinism {
        for (q, first) in file.questions.iter().zip(&outcomes) {
            let again = pipeline.search_single_async(&ranking_request(&q.text)).await?;
            if ranked_docs(&corpus, &again) != first.ranked {
                return Err(RunError::Environment(format!("non-deterministic ranking for question {}", q.id)));
            }
        }
    }

    let results: Vec<_> = outcomes.iter().map(metrics::score).collect();
    let latencies: Vec<f64> = outcomes.iter().map(|o| o.latency_ms).collect();
    let dense = !target.is_mock();
    Ok(RunResult {
        schema: SCHEMA,
        config: RunConfig {
            config_id: config_id(&target),
            corpus_version: corpus.version()?,
            questions_version,
            retrievers: Retrievers { lexical: true, dense, reranker: true },
            embedding_target: target.key(),
            reranker_model: DEFAULT_RERANKER_ID.to_string(),
            code_revision: code_revision(),
        },
        metrics: metrics::aggregate(&results),
        latency_ms: metrics::latency(&latencies),
        questions: results,
    })
}

/// The configuration id of a run: `lexical-rerank` without dense, `dense-<provider>-<model>-rerank` with it.
pub fn config_id(target: &EmbeddingTarget) -> String {
    if target.is_mock() {
        "lexical-rerank".to_string()
    } else {
        format!("dense-{}-{}-rerank", target.provider, target.model)
    }
}

/// The embedding model of a run: the mock (no dense) by default, or a catalog model given by id,
/// optionally as `<provider>/<id>` (e.g. `local/multilingual-e5-small`, `gemini/gemini-embedding-2`).
fn expected_model(embedding: Option<&str>) -> Result<&'static EmbeddingModelSpec, String> {
    let Some(key) = embedding else {
        return find_embedding_model("mock").ok_or_else(|| "the mock model is missing from the catalog".to_string());
    };
    let id = key.trim().split_once('/').map(|(_, id)| id).unwrap_or(key.trim());
    let id = id.split('@').next().unwrap_or(id);
    let spec = find_embedding_model(id).ok_or_else(|| {
        let ids: Vec<&str> = any_context_core_rs::ingestion::selectable_models().map(|m| m.id).collect();
        format!("--embedding '{key}' is not in the embedding catalog ({})", ids.join(", "))
    })?;
    if spec.is_mock() {
        return Err("--embedding mock is the default run; leave the option out".into());
    }
    Ok(spec)
}

async fn index(db: &Arc<NativeConfigDb>, lance: &Arc<NativeLanceStore>, corpus: &Corpus, spec: &EmbeddingModelSpec) -> Result<(), String> {
    let target = spec.target();
    let orchestrator = NativeSyncOrchestrator::new(lance.clone(), db.clone());
    let options = SyncOptions {
        workspace: WORKSPACE.to_string(),
        force: true,
        target_folder: None,
        verbose: false,
        embedding_model: Some(spec.id.to_string()),
    };
    let started = Instant::now();
    let result = orchestrator.run(&options).await?;
    if !result.errors.is_empty() {
        return Err(format!("indexing failed: {}", result.errors.join("; ")));
    }
    if result.indexed_files != corpus.len() {
        return Err(format!("indexing covered {} of {} corpus files", result.indexed_files, corpus.len()));
    }
    // The core never switches model (IMP-028); this guards the benchmark's own configuration.
    let recorded = db.get_workspace_embedding_target(WORKSPACE).ok().flatten().flatten();
    if recorded.as_deref() != Some(target.key().as_str()) {
        return Err(format!(
            "the index was embedded with {} instead of {} (missing key or provider error?)",
            recorded.as_deref().unwrap_or("nothing"),
            target.key()
        ));
    }
    eprintln!(
        "indexed {} files, {} chunks in {} ms (embedding {})",
        result.indexed_files,
        result.chunks_created,
        started.elapsed().as_millis(),
        target.key()
    );
    Ok(())
}

/// Passages of documents whose text only the ingestion can read (PDF) are checked on their chunks.
async fn check_indexed_passages(lance: &NativeLanceStore, corpus: &Corpus, file: &QuestionFile) -> Result<(), RunError> {
    let mut errors = Vec::new();
    for q in &file.questions {
        for r in &q.relevant {
            let Some(passage) = &r.passage else { continue };
            if corpus.plain_text(&r.doc).is_some() {
                continue;
            }
            let name = r.doc.rsplit('/').next().unwrap_or(&r.doc).replace('\'', "''");
            let rows = lance
                .search_metadata_async(&format!("file_name = '{name}'"), 10_000, Some(WORKSPACE), Some(TABLE))
                .await
                .map_err(RunError::Environment)?;
            let text = rows
                .iter()
                .filter(|row| corpus.relative(&row.file_path).as_deref() == Some(r.doc.as_str()))
                .map(|row| normalize_whitespace(&row.text))
                .collect::<Vec<_>>()
                .join(" ");
            if !text.contains(&normalize_whitespace(passage)) {
                errors.push(LabelError {
                    question: q.id.clone(),
                    reason: format!("passage not found in the indexed text of '{}': \"{passage}\"", r.doc),
                });
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(RunError::Labels(errors))
    }
}

async fn load_reranker() -> Result<Arc<actx_onnx::OnnxReranker>, String> {
    let spec = OnnxModelManager::find_spec(DEFAULT_RERANKER_ID).ok_or("the default reranker is not in the catalog")?;
    if !OnnxModelManager::is_installed(spec) {
        eprintln!("downloading the verified reranker {} ...", spec.id);
        OnnxModelManager::download_model(spec).await.map_err(|e| format!("reranker download: {e}"))?;
    }
    let reranker = actx_onnx::OnnxReranker::new(&OnnxModelManager::model_path(spec));
    let started = Instant::now();
    reranker.ensure_loaded().map_err(|e| format!("reranker load (is ORT_DYLIB_PATH set?): {e}"))?;
    if !reranker.is_neural_active() {
        return Err("the reranker loaded but is not active; a run without it would not be comparable".into());
    }
    eprintln!("reranker {} loaded in {} ms", spec.id, started.elapsed().as_millis());
    Ok(Arc::new(reranker))
}

/// Hardware profile of the benchmark: the standard one, so the reranker depth is the Fast default.
const PROFILE: ProfileTier = ProfileTier::SmartBalanced;

/// The product's Fast request, without `Global` (AnyContext's own documentation is not benchmarked)
/// and without the reranking time budget, so the depth never depends on the machine's speed.
pub fn delivered_request(query: &str) -> HybridSearchRequest {
    let mut req = fast_request(query, WORKSPACE, PROFILE);
    req.target_workspaces.clear();
    req.rerank_budget_ms = None;
    req
}

/// The Fast request opened to the top 10 and without the density cut, for the ranking metrics.
pub fn ranking_request(query: &str) -> HybridSearchRequest {
    let mut req = delivered_request(query);
    req.top_k = DEPTH;
    req.max_density_chars = usize::MAX;
    req
}

async fn ask(pipeline: &NativeHybridPipeline, corpus: &Corpus, q: &Question) -> Result<Outcome, String> {
    let output = pipeline.search_with_outcome(&ranking_request(&q.text)).await?;
    // A ranking the cross-encoder did not judge is not comparable with the baseline.
    if let RerankOutcome::Unavailable(cause) = output.rerank {
        return Err(format!("reranker unavailable for question {}: {cause}", q.id));
    }
    let ranking = output.results;
    let started = Instant::now();
    let delivered = pipeline.search_single_async(&delivered_request(&q.text)).await?;
    let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
    Ok(Outcome {
        id: q.id.clone(),
        lang: q.lang.clone(),
        category: q.category.clone(),
        negative: q.is_negative(),
        relevant: q.relevant.iter().map(|r| (r.doc.clone(), r.grade)).collect(),
        passages: q.relevant.iter().filter_map(|r| r.passage.clone()).collect(),
        ranked: ranked_docs(corpus, &ranking),
        ranked_chunks: ranking.iter().map(|c| c.text.clone()).collect(),
        delivered: ranked_docs(corpus, &delivered).into_iter().map(|d| d.doc).collect(),
        latency_ms,
    })
}

/// Corpus documents of a result list in rank order, each at its best position.
fn ranked_docs(corpus: &Corpus, results: &[HybridSearchResult]) -> Vec<RankedDoc> {
    let mut seen = HashSet::new();
    results
        .iter()
        .filter_map(|c| corpus.relative(&c.file_path).map(|doc| (doc, c.score)))
        .filter(|(doc, _)| seen.insert(doc.clone()))
        .map(|(doc, score)| RankedDoc { doc, score })
        .collect()
}

fn code_revision() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_follow_the_product_fast_request() {
        let product = fast_request("q", WORKSPACE, PROFILE);
        let delivered = delivered_request("q");
        assert!(delivered.target_workspaces.is_empty());
        assert_eq!((delivered.top_k, delivered.candidate_pool_k, delivered.max_density_chars), (product.top_k, product.candidate_pool_k, product.max_density_chars));
        let ranking = ranking_request("q");
        assert_eq!((ranking.top_k, ranking.max_density_chars), (DEPTH, usize::MAX));
        assert_eq!(ranking.candidate_pool_k, product.candidate_pool_k);
        assert_eq!((ranking.rerank_depth, ranking.rerank_budget_ms), (product.rerank_depth, None));
    }

    #[test]
    fn config_ids_name_the_retrievers() {
        assert_eq!(config_id(&expected_model(None).unwrap().target()), "lexical-rerank");
        let gemini = expected_model(Some("gemini/gemini-embedding-2")).unwrap();
        assert_eq!(config_id(&gemini.target()), "dense-gemini-gemini-embedding-2-rerank");
        let e5 = expected_model(Some("local/multilingual-e5-small")).unwrap();
        assert_eq!(config_id(&e5.target()), "dense-local-multilingual-e5-small-rerank");
        assert_eq!(expected_model(Some("multilingual-e5-small")).unwrap(), e5);
        assert!(expected_model(Some("gemini")).is_err());
        assert!(expected_model(Some("mock/mock-embed")).is_err());
        assert!(expected_model(None).unwrap().is_mock());
    }

    #[test]
    fn the_sandbox_redirects_every_default_path_and_cleans_up() {
        let root;
        {
            let sandbox = Sandbox::enter(None).unwrap();
            root = sandbox.root.clone();
            assert_eq!(any_context_core_rs::storage::get_default_settings_db_path(), sandbox.settings_db());
            assert_eq!(any_context_core_rs::storage::get_default_lancedb_path(), sandbox.lancedb());
            assert_eq!(any_context_core_rs::storage::get_default_models_dir(), root.join("models"));
            assert!(any_context_core_rs::storage::get_default_logs_dir().starts_with(std::env::temp_dir()));
        }
        assert!(!root.exists(), "the sandbox is removed with its guard");
    }
}
