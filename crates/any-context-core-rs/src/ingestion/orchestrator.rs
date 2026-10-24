//! 100% Native Rust Synchronization & Ingestion Orchestrator for AnyContext.
//!
//! Replaces legacy Python `unified_sync.py` and `local_folder_ingestor.py` pipelines.
//! Delivers sub-30ms incremental diffing, native AST/Markdown/Spreadsheet chunking,
//! batched vector embedding (a failed call fails the file for retry), Okapi BM25 inverted index updates,
//! and atomic SQLite telemetry. Zero Python dependencies.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use sha2::{Digest, Sha256};

use crate::ingestion::embedding::{decide_reindex, resolve_embedding_provider, EmbeddingTarget, ReindexDecision};
use crate::ingestion::enricher::NativeContextualEnricher;
use crate::ingestion::model_router::IngestionModelRouter;
use crate::ingestion::router::IngestionRouter;
use crate::ingestion::scanner::WorkspaceScanner;
use crate::ingestion::secrets::redact_secrets;
use crate::models::{ChunkPayload, SemanticEnvelope};
use crate::retrieval::bm25::BM25Index;
use crate::storage::lancedb::{NativeLanceStore, VectorRecord, DEFAULT_TABLE_NAME, DEFAULT_VECTOR_DIM};
use crate::storage::sqlite::NativeConfigDb;

use actx_lm::error::LmError;
use actx_lm::traits::LmProvider;

/// Why a file or page could not be embedded and stored. A failed file is not recorded in the
/// stat cache, so the next sync retries it (ADR-0146); its chunks stay in BM25 meanwhile.
#[derive(Debug)]
pub(crate) enum EmbedFailure {
    /// The embedding call failed.
    Provider(LmError),
    /// The provider answered with fewer (or more) vectors than chunks.
    ShortAnswer { expected: usize, got: usize },
    /// The LanceDB upsert failed.
    Store(String),
    /// Not attempted: the provider breaker of this pass is open.
    Skipped,
}

impl std::fmt::Display for EmbedFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedFailure::Provider(e) => write!(f, "embedding failed: {e}"),
            EmbedFailure::ShortAnswer { expected, got } => {
                write!(f, "embedding failed: expected {expected} vectors, got {got}")
            }
            EmbedFailure::Store(e) => write!(f, "upsert failed: {e}"),
            EmbedFailure::Skipped => write!(f, "embedding skipped: the provider failed on earlier files of this sync"),
        }
    }
}

/// Files and pages of one pass that could not be embedded and stored, for the status summary.
#[derive(Debug, Default)]
struct EmbeddingFailures {
    count: usize,
    first: Option<String>,
}

impl EmbeddingFailures {
    fn record(&mut self, message: &str) {
        self.count += 1;
        self.first.get_or_insert_with(|| message.to_string());
    }

    fn summary(&self) -> Option<String> {
        self.first.as_deref().map(|first| embedding_failure_summary(self.count, first))
    }
}

/// Consecutive failed files after which a sync pass stops calling the embedding provider.
const BREAKER_THRESHOLD: usize = 3;
/// Wait before the single retry of a rate-limited embedding call, when the provider gives none.
const RATE_LIMIT_DEFAULT_WAIT_SECS: u64 = 2;
/// Longest wait honored before that retry.
const RATE_LIMIT_MAX_WAIT_SECS: u64 = 30;

/// Stops a sync pass from calling a provider that keeps failing: an authentication error opens
/// it at once, otherwise `BREAKER_THRESHOLD` consecutive failed files do (ADR-0146). Files met
/// while it is open still go to BM25 and fail as `Skipped`, so the next sync retries them.
#[derive(Debug, Default)]
pub(crate) struct EmbedBreaker {
    consecutive_failures: usize,
    open: bool,
}

impl EmbedBreaker {
    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    fn record_failure(&mut self, unauthorized: bool) {
        self.consecutive_failures += 1;
        if unauthorized || self.consecutive_failures >= BREAKER_THRESHOLD {
            self.open = true;
        }
    }
}

/// Embeds `texts`, retrying once after a rate limit (honoring `retry_after_secs`, capped).
pub async fn embed_with_retry(provider: &Arc<dyn LmProvider>, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
    match provider.embed(model, texts).await {
        Err(LmError::RateLimit { retry_after_secs, .. }) => {
            let wait = retry_after_secs.unwrap_or(RATE_LIMIT_DEFAULT_WAIT_SECS).min(RATE_LIMIT_MAX_WAIT_SECS);
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
            provider.embed(model, texts).await
        }
        other => other,
    }
}

/// Configuration options for workspace synchronization.
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Target workspace identifier (e.g. "Default")
    pub workspace: String,
    /// Whether to force a full re-index (purging existing workspace chunks)
    pub force: bool,
    /// Optional folder to scope indexing to
    pub target_folder: Option<String>,
    /// Verbose logging output
    pub verbose: bool,
    /// Explicit model or provider override
    pub model: Option<String>,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            workspace: "Default".to_string(),
            force: false,
            target_folder: None,
            verbose: false,
            model: None,
        }
    }
}

/// Results emitted upon completion of a synchronization pass.
#[derive(Debug, Clone, Default)]
pub struct SyncResult {
    pub workspace: String,
    pub is_up_to_date: bool,
    pub total_files: usize,
    pub indexed_files: usize,
    pub chunks_created: usize,
    pub chunks_dropped_quality: usize,
    pub documents_enriched_ai: usize,
    pub quality_pass_rate: f32,
    pub deleted_files: usize,
    pub renamed_files: usize,
    pub errors: Vec<String>,
    pub duration_ms: u128,
    pub cancelled: bool,
    /// Chunks indexed before encryption at rest that this pass encrypted in place.
    pub chunks_encrypted_legacy: usize,
    /// Set when the embedding target changed and the whole workspace was reindexed (ADR-0138).
    pub embedding_reindex: Option<String>,
    /// Secret values replaced by `[SECRET REDACTED]` in the files indexed by this pass (ADR-0149).
    pub secrets_redacted: usize,
    /// Files an earlier version indexed with secrets; this pass purged and reindexed them.
    pub secrets_repaired_files: usize,
    /// Files and pages whose embedding or upsert failed; the next sync retries them (ADR-0146).
    pub embedding_failures: usize,
    /// The provider breaker of this pass opened, so later embedding calls were skipped.
    pub embedding_provider_unavailable: bool,
}

/// Version of the secret detection rules; a workspace indexed under another version is scanned
/// once for stored secrets (ADR-0149). Bump it when a rule change must repair existing indexes.
const SECRET_REDACTION_VERSION: &str = "1";

/// Status summary written to `workspace_sync_status.error` after a pass with embedding
/// failures. The text before the first `"; "` is the headline (see `sync_error_headline`).
pub fn embedding_failure_summary(failures: usize, first_error: &str) -> String {
    let noun = if failures == 1 { "file" } else { "files" };
    format!("{failures} {noun} not embedded; they are retried on the next sync; first error: {first_error}")
}

/// Short form of a stored sync error for one-line surfaces such as the TUI footer.
pub fn sync_error_headline(error: &str) -> &str {
    error.split("; ").next().unwrap_or(error)
}

/// Pure Rust Ingestion & Vectorization Orchestrator.
pub struct NativeSyncOrchestrator {
    router: IngestionRouter,
    model_router: Arc<IngestionModelRouter>,
    scanner: WorkspaceScanner,
    lance_store: Arc<NativeLanceStore>,
    db: Arc<NativeConfigDb>,
    /// Test seam: replaces the resolved embedding provider (the target stays the resolved one).
    embedding_override: Option<Arc<dyn LmProvider>>,
}

impl NativeSyncOrchestrator {
    /// Initializes orchestrator using system defaults for LanceDB and SQLite.
    pub fn new_default() -> Result<Self, String> {
        let db = Arc::new(NativeConfigDb::open_default().map_err(|e| e.to_string())?);
        let lance_store = Arc::new(NativeLanceStore::open_default()?);
        Ok(Self {
            router: IngestionRouter::default(),
            model_router: Arc::new(IngestionModelRouter::default()),
            scanner: WorkspaceScanner::default(),
            lance_store,
            db,
            embedding_override: None,
        })
    }

    /// Initializes orchestrator with explicitly supplied database and store instances.
    pub fn new(lance_store: Arc<NativeLanceStore>, db: Arc<NativeConfigDb>) -> Self {
        Self {
            router: IngestionRouter::default(),
            model_router: Arc::new(IngestionModelRouter::default()),
            scanner: WorkspaceScanner::default(),
            lance_store,
            db,
            embedding_override: None,
        }
    }

    pub fn with_model_router(mut self, model_router: Arc<IngestionModelRouter>) -> Self {
        self.model_router = model_router;
        self
    }

    #[cfg(test)]
    fn with_embedding_provider(mut self, provider: Arc<dyn LmProvider>) -> Self {
        self.embedding_override = Some(provider);
        self
    }

    /// Synchronous convenience entrypoint that manages its own Tokio execution if needed.
    pub fn run_sync(&self, options: &SyncOptions) -> Result<SyncResult, String> {
        if tokio::runtime::Handle::try_current().is_ok() {
            // Already inside a Tokio runtime: spawn worker thread to avoid blocking or panic
            let orchestrator = Self {
                router: self.router.clone(),
                model_router: self.model_router.clone(),
                scanner: self.scanner.clone(),
                lance_store: self.lance_store.clone(),
                db: self.db.clone(),
                embedding_override: self.embedding_override.clone(),
            };
            let opts = options.clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                rt.block_on(orchestrator.run(&opts))
            })
            .join()
            .map_err(|_| "Sync orchestrator thread panicked".to_string())?
        } else {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            rt.block_on(self.run(options))
        }
    }

    /// Primary asynchronous synchronization pipeline.
    pub async fn run(&self, options: &SyncOptions) -> Result<SyncResult, String> {
        let start_time = Instant::now();
        let ws = options.workspace.trim();
        let pid = std::process::id();

        // 1. Initial Telemetry: Mark workspace synchronization as started
        let _ = self.db.update_sync_status(
            ws,
            true,
            Some(pid),
            0,
            0,
            "scanning",
            Some("Scanning filesystem..."),
            None,
        );

        // Rows indexed before every write was encrypted are encrypted in place (ADR-0135).
        // A failure is reported in the result and retried on the next pass.
        let mut legacy_encryption_errors = Vec::new();
        // Tables created before ADR-0142 get the nullable provenance columns in place; old rows
        // keep null provenance until their file changes or a forced sync.
        let provenance_columns_added = match self.lance_store.ensure_provenance_columns(None) {
            Ok(added) => added,
            Err(e) => {
                legacy_encryption_errors.push(e);
                false
            }
        };
        let chunks_encrypted_legacy = match self.lance_store.encrypt_plaintext_rows(None) {
            Ok(report) => report.rows_encrypted,
            Err(e) => {
                legacy_encryption_errors.push(format!("Encrypting chunks stored in plaintext failed: {e}"));
                0
            }
        };
        // Files indexed with secrets by an earlier version lose their chunks and cache entry,
        // so the diff below reindexes them redacted in this same pass (ADR-0149).
        let secrets_repaired_files = match self.repair_indexed_secrets(ws) {
            Ok(paths) => paths,
            Err(e) => {
                legacy_encryption_errors.push(format!("Repairing chunks indexed with secrets failed: {e}"));
                Vec::new()
            }
        };

        // 1b. Embedding target: vectors from another target than the current one are purged and
        // the whole workspace is re-embedded, even for a folder-scoped pass (ADR-0138).
        let (mut lm_provider, embedding_target) = resolve_embedding_provider(&self.db, options.model.as_deref(), ws);
        if let Some(ref provider) = self.embedding_override {
            lm_provider = Some(provider.clone());
        }
        let embedding_model = embedding_target.model.clone();
        let reindex = self.decide_embedding_reindex(ws, &embedding_target);
        let embedding_reindex = reindex.message();
        let full_reindex = matches!(reindex, ReindexDecision::Reindex { .. });
        let force = options.force || full_reindex;
        let target_folder = if full_reindex { None } else { options.target_folder.clone() };
        if let Some(ref msg) = embedding_reindex {
            let _ = self.db.update_sync_status(ws, true, Some(pid), 0, 0, "scanning", Some(msg), None);
        }

        if options.verbose {
            if provenance_columns_added {
                println!("🧭 LanceDB: added provenance columns header_path, start_line, end_line.");
            }
            if chunks_encrypted_legacy > 0 {
                println!("🔒 Encrypted {} chunks stored before encryption at rest.", chunks_encrypted_legacy);
            }
            for path in &secrets_repaired_files {
                println!("🔐 Indexed with secrets by an earlier version, reindexed redacted: {path}");
            }
            println!("🦀 [AnyContext Native Rust Ingestion Pipeline]");
            println!("  • Workspace: {}", ws);
            println!("  • Force Reindex: {}", force);
            if let Some(ref msg) = embedding_reindex {
                println!("  • {}", msg);
            }
            if let Some(ref tf) = target_folder {
                println!("  • Scoped Folder: {}", tf);
            }
        }

        // 2. Resolve Monitored Folders
        let folders = if let Some(ref tf) = target_folder {
            vec![tf.clone()]
        } else {
            let list = self.db.get_workspace_folders(ws).unwrap_or_default();
            if list.is_empty() {
                // Default to current working directory if no explicit folders configured
                if let Ok(cwd) = std::env::current_dir() {
                    vec![cwd.to_string_lossy().to_string()]
                } else {
                    vec![]
                }
            } else {
                list
            }
        };

        // 3. Load Inverted BM25 Index
        let bm25_path = self.lance_store.db_path().join("bm25_index.bin");
        let mut bm25 = match BM25Index::load_from_file(bm25_path.to_str().unwrap_or("")) {
            Ok(idx) => idx,
            Err(_) => BM25Index::new(None, None),
        };

        // 4. Embedding provider: resolved in step 1b.

        // 5. Check if Force Mode requested
        if force {
            if options.verbose {
                println!("  • Force mode: Purging existing vectors for workspace '{}'...", ws);
            }
            if let Some(ref tf) = target_folder {
                let _ = self.lance_store.delete_by_file(tf, Some(ws), None);
                let _ = bm25.remove_by_file(tf);
            } else {
                let _ = self.lance_store.delete_by_workspace(ws, None);
                let _ = bm25.remove_by_workspace(ws);
                let _ = self.db.clear_workspace_stat_cache(ws);
            }
        }

        // 6. Query cached metadata from SQLite
        let cached_stats = if force {
            HashMap::new()
        } else {
            self.db.get_workspace_files_stat_cache(ws).unwrap_or_default()
        };

        // 7. High-speed multi-folder filesystem scan and differential calculation
        let diff = self.scanner.scan_and_diff_native(&folders, &cached_stats);

        if options.verbose {
            println!(
                "  • Scan summary: {} on disk ({} new, {} modified, {} deleted, {} renamed)",
                diff.disk_files.len(),
                diff.new_files.len(),
                diff.modified_files.len(),
                diff.deleted_files.len(),
                diff.renamed_files.len()
            );
        }

        // 8. Process zero-cost renamed/moved files
        for (old_p, new_p) in &diff.renamed_files {
            let _ = self.db.rename_cached_file(ws, old_p, new_p);
            // Delete old records in LanceDB & BM25; new location will be indexed below if modified
            let _ = self.lance_store.delete_by_file(old_p, Some(ws), None);
            let _ = bm25.remove_by_file(old_p);
        }

        // 9. Purge deleted files
        for del_p in &diff.deleted_files {
            let _ = self.lance_store.delete_by_file(del_p, Some(ws), None);
            let _ = bm25.remove_by_file(del_p);
            let _ = self.db.delete_file_stat_cache(ws, del_p);
        }

        // 10. Purge chunks of modified files before re-indexing (Purge-Before-Embed)
        for mod_p in &diff.modified_files {
            let _ = self.lance_store.delete_by_file(mod_p, Some(ws), None);
            let _ = bm25.remove_by_file(mod_p);
        }

        // 11. Early Exit if 100% Up to Date
        let mut files_to_index: Vec<String> = diff.new_files.clone();
        files_to_index.extend(diff.modified_files.clone());
        // Include renamed files in index list so their content is re-associated with new path
        for (_, new_p) in &diff.renamed_files {
            if !files_to_index.contains(new_p) {
                files_to_index.push(new_p.clone());
            }
        }

        let web_urls = if target_folder.is_none() {
            self.db.get_workspace_web_urls(ws).unwrap_or_default()
        } else {
            vec![]
        };

        let enricher = NativeContextualEnricher::new(self.db.clone());

        if files_to_index.is_empty() && web_urls.is_empty() {
            if !diff.deleted_files.is_empty() || !diff.renamed_files.is_empty() {
                let _ = bm25.save_to_file(bm25_path.to_str().unwrap_or(""));
                let _ = self.db.record_sync_ledger(
                    ws,
                    &diff.deleted_files,
                    &diff.new_files,
                    &diff.modified_files,
                );
            }

            let _ = self.db.update_sync_status(
                ws,
                false,
                None,
                diff.disk_files.len(),
                diff.disk_files.len(),
                "idle",
                None,
                None,
            );
            if options.verbose {
                if !diff.missing_folders.is_empty() {
                    println!("⚠️ Workspace '{}' has unreachable folder(s): {}", ws, diff.missing_folders.join(", "));
                } else if diff.is_up_to_date {
                    println!("✔ Workspace '{}' is 100% up-to-date (0 changes).", ws);
                } else {
                    println!("✔ Workspace '{}' synchronized: {} files purged.", ws, diff.deleted_files.len());
                }
            }
            self.record_embedding_target(ws, &reindex, &embedding_target);
            return Ok(SyncResult {
                workspace: ws.to_string(),
                is_up_to_date: diff.is_up_to_date,
                total_files: diff.disk_files.len(),
                indexed_files: 0,
                chunks_created: 0,
                chunks_dropped_quality: 0,
                documents_enriched_ai: 0,
                quality_pass_rate: 1.0,
                deleted_files: diff.deleted_files.len(),
                renamed_files: diff.renamed_files.len(),
                errors: legacy_encryption_errors,
                duration_ms: start_time.elapsed().as_millis(),
                cancelled: false,
                chunks_encrypted_legacy,
                embedding_reindex: embedding_reindex.clone(),
                secrets_redacted: 0,
                secrets_repaired_files: secrets_repaired_files.len(),
                embedding_failures: 0,
                embedding_provider_unavailable: false,
            });
        }

        let total_files = files_to_index.len();
        let mut indexed_count = 0;
        let mut total_chunks_created = 0;
        let mut total_secrets_redacted = 0;
        let mut total_chunks_dropped_quality = 0;
        let mut total_documents_enriched_ai = 0;
        let mut errors = legacy_encryption_errors;

        // 12. Main Indexing Loop
        let mut breaker = EmbedBreaker::default();
        let mut failures = EmbeddingFailures::default();
        for (idx, file_path) in files_to_index.iter().enumerate() {
            // Check for cooperative cancellation
            if let Ok(Some(st)) = self.db.get_sync_status(ws) {
                if !st.is_syncing || st.stage == "cancelled" {
                    if options.verbose {
                        println!("🛑 Synchronization cancelled by user request.");
                    }
                    let _ = self.db.update_sync_status(
                        ws,
                        false,
                        None,
                        idx,
                        total_files,
                        "cancelled",
                        None,
                        None,
                    );
                    let pass_rate = if total_chunks_created + total_chunks_dropped_quality > 0 {
                        (total_chunks_created as f32) / ((total_chunks_created + total_chunks_dropped_quality) as f32)
                    } else {
                        1.0
                    };
                    return Ok(SyncResult {
                        workspace: ws.to_string(),
                        is_up_to_date: false,
                        total_files,
                        indexed_files: indexed_count,
                        chunks_created: total_chunks_created,
                        chunks_dropped_quality: total_chunks_dropped_quality,
                        documents_enriched_ai: total_documents_enriched_ai,
                        quality_pass_rate: pass_rate,
                        deleted_files: diff.deleted_files.len(),
                        renamed_files: diff.renamed_files.len(),
                        errors,
                        duration_ms: start_time.elapsed().as_millis(),
                        cancelled: true,
                        chunks_encrypted_legacy,
                        embedding_reindex: embedding_reindex.clone(),
                        secrets_redacted: total_secrets_redacted,
                        secrets_repaired_files: secrets_repaired_files.len(),
                        embedding_failures: failures.count,
                        embedding_provider_unavailable: breaker.is_open(),
                    });
                }
            }

            let path_obj = Path::new(file_path);
            let file_name = path_obj.file_name().and_then(|n| n.to_str()).unwrap_or("document").to_string();

            // Progress Telemetry
            let _ = self.db.update_sync_status(
                ws,
                true,
                Some(pid),
                idx + 1,
                total_files,
                "files",
                Some(&file_name),
                None,
            );

            if options.verbose {
                println!("  [{}/{}] Indexing: {}", idx + 1, total_files, file_name);
            }

            // A file that failed on an earlier pass may still have BM25 chunks under an older
            // content hash: drop them so a retry never leaves orphans behind.
            bm25.remove_by_exact_file(file_path);

            // Chunk the file with its secret values redacted (ADR-0149)
            let raw_chunks = match self.router.chunk_file_redacted(file_path) {
                Ok(redacted) => {
                    if redacted.secrets_redacted > 0 && options.verbose {
                        println!("  🔐 {} secret value(s) redacted in {}", redacted.secrets_redacted, file_name);
                    }
                    total_secrets_redacted += redacted.secrets_redacted;
                    redacted.chunks
                }
                Err(e) => {
                    let err_msg = format!("Failed to chunk '{}': {}", file_path, e);
                    errors.push(err_msg);
                    continue;
                }
            };

            // Ingestion ModelRouter: Quality Gate & Document AI Triage
            let (mut approved_chunks, ai_candidates, dropped_noise) = self.model_router.triage_chunks(raw_chunks);
            total_chunks_dropped_quality += dropped_noise;

            if !ai_candidates.is_empty() {
                total_documents_enriched_ai += 1;
                let ai_chunks = self.model_router.process_document_ai_candidates(
                    ai_candidates,
                    lm_provider.as_ref(),
                ).await;
                approved_chunks.extend(ai_chunks);
            }

            let chunks = approved_chunks;

            let (mtime, size) = diff.disk_files.get(file_path).copied().unwrap_or((0.0, 0));
            let content_hash = compute_file_sha256(path_obj).unwrap_or_else(|_| "hash_err".to_string());

            if chunks.is_empty() {
                // File was empty, whitespace, or filtered as noise; record in SQLite cache so it doesn't get repeatedly checked
                let _ = self.db.upsert_file_stat_cache(ws, file_path, mtime, size, Some(&content_hash));
                indexed_count += 1;
                continue;
            }

            // Extract contextual envelope; its summary and keywords are prefixed to every chunk,
            // so the raw file is redacted too (ADR-0149)
            let doc_sample = if let Ok(full_text) = std::fs::read_to_string(path_obj) {
                redact_secrets(&full_text).text
            } else {
                chunks.iter().take(10).map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n")
            };
            let envelope = enricher.extract_envelope(&doc_sample, &file_name, Some(file_path), None);

            // Vectorize and upsert chunks in batches
            match self.embed_and_upsert_chunks(
                &chunks,
                Some(&envelope),
                &enricher,
                ws,
                file_path,
                &file_name,
                &content_hash,
                mtime,
                &lm_provider,
                &embedding_model,
                &mut bm25,
                &mut breaker,
            ).await {
                Ok(num_chunks) => {
                    total_chunks_created += num_chunks;
                    indexed_count += 1;
                    let _ = self.db.upsert_file_stat_cache(ws, file_path, mtime, size, Some(&content_hash));
                }
                Err(e) => {
                    let err_msg = format!("Embedding/upsert error on '{}': {}", file_path, e);
                    failures.record(&err_msg);
                    errors.push(err_msg);
                }
            }
        }

        // 13. Synchronize Web Documentation Portals (if any)
        if !web_urls.is_empty() {
            self.sync_web_portals(
                &web_urls,
                ws,
                pid,
                &enricher,
                &lm_provider,
                &embedding_model,
                &mut bm25,
                &mut breaker,
                options.verbose,
                &mut errors,
                &mut failures,
            ).await;
        }

        // 14. Persist Inverted BM25 Index to Disk
        let _ = bm25.save_to_file(bm25_path.to_str().unwrap_or(""));

        // 15. Record Sync Ledger Diff in SQLite
        let _ = self.db.record_sync_ledger(
            ws,
            &diff.deleted_files,
            &diff.new_files,
            &diff.modified_files,
        );

        // 15. Finalize Telemetry: Set sync state to idle, with the embedding failures of this
        // pass (a clean pass clears an earlier summary).
        let failure_summary = failures.summary();
        let _ = self.db.update_sync_status(
            ws,
            false,
            None,
            total_files,
            total_files,
            "idle",
            None,
            failure_summary.as_deref(),
        );

        let duration_ms = start_time.elapsed().as_millis();
        let pass_rate = if total_chunks_created + total_chunks_dropped_quality > 0 {
            (total_chunks_created as f32) / ((total_chunks_created + total_chunks_dropped_quality) as f32)
        } else {
            1.0
        };

        if options.verbose {
            println!(
                "✔ Synchronization completed in {}ms: {} files indexed, {} chunks created, {} noise filtered (pass rate: {:.1}%), {} deleted.",
                duration_ms, indexed_count, total_chunks_created, total_chunks_dropped_quality, pass_rate * 100.0, diff.deleted_files.len()
            );
        }

        self.record_embedding_target(ws, &reindex, &embedding_target);

        Ok(SyncResult {
            workspace: ws.to_string(),
            is_up_to_date: false,
            total_files,
            indexed_files: indexed_count,
            chunks_created: total_chunks_created,
            chunks_dropped_quality: total_chunks_dropped_quality,
            documents_enriched_ai: total_documents_enriched_ai,
            quality_pass_rate: pass_rate,
            deleted_files: diff.deleted_files.len(),
            renamed_files: diff.renamed_files.len(),
            errors,
            duration_ms,
            cancelled: false,
            chunks_encrypted_legacy,
            embedding_reindex,
            secrets_redacted: total_secrets_redacted,
            secrets_repaired_files: secrets_repaired_files.len(),
            embedding_failures: failures.count,
            embedding_provider_unavailable: breaker.is_open(),
        })
    }

    /// Embeds a list of chunks, adds them to BM25, and upserts them to LanceDB.
    async fn embed_and_upsert_chunks(
        &self,
        chunks: &[ChunkPayload],
        envelope: Option<&SemanticEnvelope>,
        enricher: &NativeContextualEnricher,
        workspace: &str,
        file_path: &str,
        file_name: &str,
        content_hash: &str,
        mtime: f64,
        lm_provider: &Option<Arc<dyn LmProvider>>,
        embedding_model: &str,
        bm25: &mut BM25Index,
        breaker: &mut EmbedBreaker,
    ) -> Result<usize, EmbedFailure> {
        let last_mod_str = chrono::DateTime::from_timestamp(mtime as i64, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();

        // 1. Prepare texts for embedding & populate BM25 index
        let mut chunk_texts = Vec::with_capacity(chunks.len());
        let mut chunk_ids = Vec::with_capacity(chunks.len());

        let short_hash = if content_hash.len() >= 12 {
            &content_hash[..12]
        } else {
            content_hash
        };

        for (i, c) in chunks.iter().enumerate() {
            let id = format!("{}_{}", short_hash, i);
            let text_to_embed = if let Some(env) = envelope {
                enricher.apply_envelope_to_chunk(&c.text, env)
            } else {
                c.text.clone()
            };

            bm25.add_chunk(
                id.clone(),
                text_to_embed.clone(),
                file_name.to_string(),
                file_path.to_string(),
                workspace.to_string(),
                c.content_type.clone(),
                Some(last_mod_str.clone()),
            );
            chunk_texts.push(text_to_embed);
            chunk_ids.push(id);
        }

        // 2. Generate Dense Embeddings. A failed call fails the file: no pseudo-vectors are
        // stored next to real ones (ADR-0146). Without a provider, the offline fallback stays.
        let vectors = if let Some(provider) = lm_provider {
            if breaker.is_open() {
                return Err(EmbedFailure::Skipped);
            }
            match embed_with_retry(provider, embedding_model, &chunk_texts).await {
                Ok(v) if v.len() == chunks.len() => {
                    breaker.record_success();
                    v
                }
                Ok(v) => {
                    breaker.record_failure(false);
                    return Err(EmbedFailure::ShortAnswer { expected: chunks.len(), got: v.len() });
                }
                Err(e) => {
                    breaker.record_failure(matches!(e, LmError::Auth { .. }));
                    return Err(EmbedFailure::Provider(e));
                }
            }
        } else {
            generate_fallback_vectors(&chunk_texts, DEFAULT_VECTOR_DIM)
        };

        // 3. Assemble VectorRecords for LanceDB, with the chunk provenance (ADR-0142)
        let file_lines = self.router.has_file_lines(file_path);
        let mut records = Vec::with_capacity(chunks.len());
        for ((i, c), vector) in chunks.iter().enumerate().zip(vectors) {
            let doc_summary = envelope.map(|env| env.summary.clone());
            let keywords = envelope.map(|env| env.keywords.join(", "));
            let (start_line, end_line) = if file_lines && c.start_line >= 1 && c.start_line <= c.end_line {
                (u32::try_from(c.start_line).ok(), u32::try_from(c.end_line).ok())
            } else {
                (None, None)
            };

            records.push(VectorRecord {
                id: chunk_ids[i].clone(),
                vector,
                text: c.text.clone(),
                file_name: file_name.to_string(),
                file_path: file_path.to_string(),
                workspace: workspace.to_string(),
                last_modified: Some(last_mod_str.clone()),
                content_type: Some(c.content_type.clone()),
                document_summary: doc_summary,
                keywords,
                content_hash: Some(content_hash.to_string()),
                header_path: c.header_path.clone().filter(|h| !h.trim().is_empty()),
                start_line,
                end_line,
            });
        }

        // 4. Batch upsert into LanceDB
        self.lance_store
            .upsert_records(records, Some(DEFAULT_TABLE_NAME), Some(DEFAULT_VECTOR_DIM))
            .map_err(EmbedFailure::Store)?;

        Ok(chunks.len())
    }

    /// Crawls and vectorizes configured documentation web portals using recursive NativeWebCrawler.
    async fn sync_web_portals(
        &self,
        urls: &[String],
        workspace: &str,
        pid: u32,
        enricher: &NativeContextualEnricher,
        lm_provider: &Option<Arc<dyn LmProvider>>,
        embedding_model: &str,
        bm25: &mut BM25Index,
        breaker: &mut EmbedBreaker,
        verbose: bool,
        errors: &mut Vec<String>,
        failures: &mut EmbeddingFailures,
    ) {
        let crawler = match crate::ingestion::crawler::NativeWebCrawler::new_default() {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("Failed to initialize crawler: {e}"));
                return;
            }
        };

        for (u_idx, url) in urls.iter().enumerate() {
            let _ = self.db.update_sync_status(
                workspace,
                true,
                Some(pid),
                u_idx + 1,
                urls.len(),
                "web",
                Some(url),
                None,
            );

            if verbose {
                println!("  🌐 Crawling Web Portal: {}", url);
            }

            let pages = match crawler.crawl(url).await {
                Ok(p) => p,
                Err(e) => {
                    if verbose {
                        println!("  ⚠️ Crawl error on '{}': {}", url, e);
                    }
                    continue;
                }
            };

            for page in pages {
                if let Err(e) = self.index_web_page(
                    &page,
                    workspace,
                    enricher,
                    lm_provider,
                    embedding_model,
                    bm25,
                    breaker,
                ).await {
                    let err_msg = format!("Failed to index page '{}': {}", page.url, e);
                    failures.record(&err_msg);
                    errors.push(err_msg);
                }
            }
        }
    }

    /// Indexes one crawled page. Returns `Ok(false)` when the page was skipped (no chunks, or
    /// already indexed with the same content hash) and `Ok(true)` when it was (re)indexed.
    /// A changed page first loses its previous chunks, so re-crawls never accumulate duplicates.
    async fn index_web_page(
        &self,
        page: &crate::ingestion::crawler::CrawledPage,
        workspace: &str,
        enricher: &NativeContextualEnricher,
        lm_provider: &Option<Arc<dyn LmProvider>>,
        embedding_model: &str,
        bm25: &mut BM25Index,
        breaker: &mut EmbedBreaker,
    ) -> Result<bool, String> {
        let chunks = self.router.chunk_text_native(&page.url, &page.text)?;
        if chunks.is_empty() {
            return Ok(false);
        }

        let mut hasher = Sha256::new();
        hasher.update(page.text.as_bytes());
        let hash_hex = format!("{:x}", hasher.finalize());

        // Unchanged page: its chunks are already indexed under this URL and content hash.
        let clean_url = page.url.replace('\'', "''");
        let unchanged_clause = format!("file_path = '{clean_url}' AND content_hash = '{hash_hex}'");
        let already_indexed = self
            .lance_store
            .search_metadata_async(&unchanged_clause, 1, Some(workspace), None)
            .await
            .map(|hits| hits.iter().any(|h| h.workspace == workspace))
            .unwrap_or(false);
        if already_indexed {
            return Ok(false);
        }

        // New or changed page: drop its previous chunks (exact URL only, never sub-pages).
        self.lance_store.delete_by_exact_path(&page.url, Some(workspace), None)?;
        bm25.remove_by_exact_file(&page.url);

        let envelope = enricher.extract_envelope(&page.text, &page.title, None, Some(&page.url));

        let page_mtime = if let Some(ref lm) = page.lastmod {
            chrono::DateTime::parse_from_rfc3339(lm)
                .map(|dt| dt.timestamp() as f64)
                .or_else(|_| {
                    chrono::NaiveDate::parse_from_str(lm, "%Y-%m-%d")
                        .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() as f64)
                })
                .unwrap_or_else(|_| chrono::Utc::now().timestamp() as f64)
        } else {
            chrono::Utc::now().timestamp() as f64
        };

        self.embed_and_upsert_chunks(
            &chunks,
            Some(&envelope),
            enricher,
            workspace,
            &page.url,
            &page.title,
            &hash_hex,
            page_mtime,
            lm_provider,
            embedding_model,
            bm25,
            breaker,
        ).await.map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Decides whether the workspace's stored vectors match the current embedding target.
    /// A workspace without a row in SQLite cannot record a target and is left as it is.
    /// One-time repair of a workspace indexed before secret redaction (ADR-0149): every file
    /// with a stored chunk, summary, keyword list or section path that holds a secret loses its
    /// rows (removed from the disk too) and its stat cache entry, so the next diff reindexes it
    /// redacted. Recorded per workspace with `SECRET_REDACTION_VERSION`, so later passes skip
    /// the scan and a future rule change can run it again. Returns the purged paths.
    fn repair_indexed_secrets(&self, workspace: &str) -> Result<Vec<String>, String> {
        let marker = format!("secret_redaction_version:{workspace}");
        if self.db.get_setting(&marker).ok().flatten().as_deref() == Some(SECRET_REDACTION_VERSION) {
            return Ok(Vec::new());
        }
        let mut affected = std::collections::BTreeSet::new();
        let clean_ws = workspace.replace('\'', "''");
        self.lance_store.for_each_row(&format!("workspace = '{clean_ws}'"), None, |row| {
            let holds_secret = [
                Some(row.text.as_str()),
                row.document_summary.as_deref(),
                row.keywords.as_deref(),
                row.header_path.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|field| redact_secrets(field).secrets_found > 0);
            if holds_secret {
                affected.insert(row.file_path.clone());
            }
        })?;
        let paths: Vec<String> = affected.into_iter().collect();
        self.lance_store.purge_paths_from_disk(workspace, &paths, None)?;
        for path in &paths {
            self.db.delete_file_stat_cache(workspace, path).map_err(|e| e.to_string())?;
        }
        self.db.set_setting(&marker, SECRET_REDACTION_VERSION).map_err(|e| e.to_string())?;
        Ok(paths)
    }

    fn decide_embedding_reindex(&self, workspace: &str, current: &EmbeddingTarget) -> ReindexDecision {
        let recorded = match self.db.get_workspace_embedding_target(workspace) {
            Ok(Some(recorded)) => recorded,
            _ => return ReindexDecision::Keep,
        };
        // A failed count is treated as indexed data, so a legacy target is never adopted blindly.
        let has_indexed_data = self
            .lance_store
            .count_records(Some(workspace), None)
            .map(|n| n > 0)
            .unwrap_or(true);
        decide_reindex(recorded.as_deref(), current, has_indexed_data)
    }

    /// Records the target once a pass completed, so a cancelled reindex is retried next time.
    fn record_embedding_target(&self, workspace: &str, decision: &ReindexDecision, target: &EmbeddingTarget) {
        if *decision != ReindexDecision::Keep {
            let _ = self.db.set_workspace_embedding_target(workspace, Some(&target.key()));
        }
    }
}

/// Reads file and computes SHA-256 hex string.
fn compute_file_sha256(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Generates normalized deterministic pseudo-vectors when operating offline without API keys.
fn generate_fallback_vectors(texts: &[String], dim: usize) -> Vec<Vec<f32>> {
    texts
        .iter()
        .map(|t| {
            let mut hasher = Sha256::new();
            hasher.update(t.as_bytes());
            let hash = hasher.finalize();

            let mut vec = Vec::with_capacity(dim);
            let hash_len = hash.len();
            for i in 0..dim {
                let byte = hash[i % hash_len];
                let val = ((byte as f32 / 255.0) - 0.5) * 0.1;
                vec.push(val);
            }
            vec
        })
        .collect()
}

/// Fast HTML tag stripper for web portal scraping without heavy external dependencies.
pub fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut in_script = false;

    let lower = html.to_lowercase();
    let bytes = html.as_bytes();
    let lower_bytes = lower.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        if !in_tag && bytes[i] == b'<' {
            if lower_bytes[i..].starts_with(b"<script") || lower_bytes[i..].starts_with(b"<style") {
                in_script = true;
            }
            in_tag = true;
            i += 1;
            continue;
        }

        if in_tag {
            if bytes[i] == b'>' {
                in_tag = false;
                if in_script {
                    if i >= 8 && lower_bytes[i - 8..i].ends_with(b"/script") {
                        in_script = false;
                    } else if i >= 7 && lower_bytes[i - 7..i].ends_with(b"/style") {
                        in_script = false;
                    }
                }
            }
            i += 1;
            continue;
        }

        if !in_script {
            let c = bytes[i] as char;
            if c == '\n' || c == '\r' || c == '\t' || c == ' ' {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            } else {
                out.push(c);
            }
        }
        i += 1;
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_file_sha256() {
        let temp = std::env::temp_dir().join(format!("test_sha_{}.txt", std::process::id()));
        std::fs::write(&temp, "AnyContext Rust Orchestrator").unwrap();
        let hash = compute_file_sha256(&temp).unwrap();
        assert_eq!(hash.len(), 64);
        let _ = std::fs::remove_file(temp);
    }

    #[test]
    fn test_generate_fallback_vectors() {
        let texts = vec!["hello world".to_string(), "any-context".to_string()];
        let vecs = generate_fallback_vectors(&texts, 1536);
        assert_eq!(vecs.len(), 2);
        assert_eq!(vecs[0].len(), 1536);
        assert_eq!(vecs[1].len(), 1536);
        assert_ne!(vecs[0], vecs[1]);
    }

    #[test]
    fn test_strip_html_tags() {
        let html = "<html><head><title>Test</title><style>body { color: red; }</style></head><body><h1>Hello World</h1><script>console.log('skip');</script><p>Paragraph text.</p></body></html>";
        let text = strip_html_tags(html);
        assert!(text.contains("Hello World"));
        assert!(text.contains("Paragraph text"));
        assert!(!text.contains("console.log"));
    }

    #[tokio::test]
    async fn test_sync_encrypts_chunks_stored_in_plaintext() {
        let temp_dir = std::env::temp_dir().join(format!("test_orch_legacy_enc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let docs_dir = temp_dir.join("docs");
        let _ = std::fs::create_dir_all(&docs_dir);

        let db = Arc::new(NativeConfigDb::open_in_memory().unwrap());
        let lance_store = Arc::new(NativeLanceStore::open(&temp_dir.join("lancedb")).unwrap());
        lance_store.write_plaintext_rows_for_test(
            None,
            crate::storage::lancedb::DEFAULT_VECTOR_DIM,
            vec![VectorRecord {
                id: "legacy_0".into(),
                vector: vec![0.5; crate::storage::lancedb::DEFAULT_VECTOR_DIM],
                text: "Chunk indexed by an older version".into(),
                file_name: "old.md".into(),
                file_path: "docs/old.md".into(),
                workspace: "LegacyWS".into(),
                last_modified: None,
                content_type: None,
                document_summary: Some("Old summary".into()),
                keywords: None,
                content_hash: Some("legacy".into()),
                header_path: None,
                start_line: None,
                end_line: None,
            }],
        );
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db);
        let opts = SyncOptions {
            workspace: "LegacyWS".to_string(),
            target_folder: Some(docs_dir.to_string_lossy().to_string()),
            model: Some("mock".to_string()),
            ..SyncOptions::default()
        };

        let first = orchestrator.run(&opts).await.unwrap();
        assert_eq!(first.chunks_encrypted_legacy, 1);
        assert!(first.errors.is_empty(), "{:?}", first.errors);
        let raw = lance_store.search_metadata("text LIKE '%older version%'", 10, None, None).unwrap();
        assert!(raw.is_empty(), "the legacy chunk must be encrypted on disk");

        let second = orchestrator.run(&opts).await.unwrap();
        assert_eq!(second.chunks_encrypted_legacy, 0);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_recrawled_web_page_never_duplicates_chunks() {
        let temp_dir = std::env::temp_dir().join(format!("test_orch_web_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let _ = std::fs::create_dir_all(&temp_dir);

        let db = Arc::new(NativeConfigDb::open_in_memory().unwrap());
        let lance_store = Arc::new(NativeLanceStore::open(&temp_dir.join("lancedb")).unwrap());
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db.clone());
        let enricher = NativeContextualEnricher::new(db.clone());
        let mut bm25 = BM25Index::new(None, None);

        let page = |url: &str, text: &str| crate::ingestion::crawler::CrawledPage {
            url: url.to_string(),
            title: "Docs".to_string(),
            text: text.to_string(),
            lastmod: None,
            etag: None,
            content_type: None,
        };
        let parent = "https://docs.example.com/guide";
        let child = "https://docs.example.com/guide/install";
        let count = |path: &str| {
            lance_store
                .search_metadata(&format!("file_path = '{path}'"), 1000, Some("WebWS"), None)
                .unwrap()
                .len()
        };

        let v1 = "# Guide\n\nThe service listens on port 8080 and reads its configuration from settings.toml at startup.";
        let sub = "# Install\n\nRun the installer, then open a new terminal and start the application with actx.";
        assert!(orchestrator.index_web_page(&page(child, sub), "WebWS", &enricher, &None, "mock", &mut bm25, &mut EmbedBreaker::default()).await.unwrap());
        assert!(orchestrator.index_web_page(&page(parent, v1), "WebWS", &enricher, &None, "mock", &mut bm25, &mut EmbedBreaker::default()).await.unwrap());
        let parent_chunks = count(parent);
        let child_chunks = count(child);
        assert!(parent_chunks > 0 && child_chunks > 0);

        // Same content again: skipped, nothing appended.
        assert!(!orchestrator.index_web_page(&page(parent, v1), "WebWS", &enricher, &None, "mock", &mut bm25, &mut EmbedBreaker::default()).await.unwrap());
        assert_eq!(count(parent), parent_chunks);

        // Changed content: old chunks replaced, and the sub-page under the same URL prefix is untouched.
        let v2 = "# Guide\n\nThe service now listens on port 9090 and reads its configuration from config.toml at startup.";
        assert!(orchestrator.index_web_page(&page(parent, v2), "WebWS", &enricher, &None, "mock", &mut bm25, &mut EmbedBreaker::default()).await.unwrap());
        let hits = lance_store
            .search_metadata(&format!("file_path = '{parent}'"), 1000, Some("WebWS"), None)
            .unwrap();
        assert!(hits.iter().all(|h| !h.text.contains("8080")), "stale chunks of the old page version remain");
        assert!(hits.iter().any(|h| h.text.contains("9090")));
        assert_eq!(count(child), child_chunks);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_orchestrator_enricher_and_ledger_integration() {
        let temp_dir = std::env::temp_dir().join(format!("test_orch_env_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let db = Arc::new(NativeConfigDb::open_in_memory().unwrap());
        let lance_store = Arc::new(NativeLanceStore::open(&temp_dir.join("lancedb")).unwrap());
        let orchestrator = NativeSyncOrchestrator::new(lance_store, db.clone());
        let enricher = NativeContextualEnricher::new(db.clone());

        let chunks = vec![ChunkPayload {
            id: "rank_0".to_string(),
            text: "fn compute_vector_rank() -> f32 { 1.0 }".to_string(),
            file_name: "rank.rs".to_string(),
            file_path: "src/rank.rs".to_string(),
            header_path: None,
            start_line: 1,
            end_line: 1,
            content_type: "rust".to_string(),
            chunk_index: 0,
        }];

        let env = enricher.extract_envelope(
            "Rust Vector Rank Module computes cosine similarity and bm25 scoring for ranking.",
            "rank.rs",
            Some("src/rank.rs"),
            None,
        );

        let mut bm25 = BM25Index::new(None, None);
        let upserted = orchestrator.embed_and_upsert_chunks(
            &chunks,
            Some(&env),
            &enricher,
            "Default",
            "src/rank.rs",
            "rank.rs",
            "mockhash123456",
            1000.0,
            &None,
            "mock",
            &mut bm25,
            &mut EmbedBreaker::default(),
        ).await.unwrap();

        assert_eq!(upserted, 1);
        assert_eq!(bm25.len(), 1);

        // Test ledger recording
        let added = vec!["src/rank.rs".to_string()];
        orchestrator.db.record_sync_ledger("Default", &[], &added, &[]).unwrap();
        let ledger = orchestrator.db.get_sync_ledger("Default").unwrap().expect("ledger exists");
        assert_eq!(ledger.2, added);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    fn chunk(id: &str, file_path: &str, header: &str, start: usize, end: usize, content_type: &str) -> ChunkPayload {
        ChunkPayload {
            id: id.to_string(),
            text: format!("Chunk {id} about request timeouts and retries."),
            file_name: Path::new(file_path).file_name().unwrap().to_string_lossy().to_string(),
            file_path: file_path.to_string(),
            header_path: Some(header.to_string()),
            start_line: start,
            end_line: end,
            content_type: content_type.to_string(),
            chunk_index: 0,
        }
    }

    async fn upsert_one(tag: &str, c: ChunkPayload, with_envelope: bool) -> crate::storage::ScoredVectorResult {
        let temp_dir = std::env::temp_dir().join(format!("test_orch_prov_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let db = Arc::new(NativeConfigDb::open_in_memory().unwrap());
        let lance_store = Arc::new(NativeLanceStore::open(&temp_dir.join("lancedb")).unwrap());
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db.clone());
        let enricher = NativeContextualEnricher::new(db);
        let env = enricher.extract_envelope("Guide to request timeouts and retry policies.", &c.file_name, Some(&c.file_path), None);
        let mut bm25 = BM25Index::new(None, None);
        let file_path = c.file_path.clone();
        orchestrator
            .embed_and_upsert_chunks(
                &[c.clone()],
                with_envelope.then_some(&env),
                &enricher,
                "Default",
                &file_path,
                &c.file_name,
                "provhash1234567",
                1000.0,
                &None,
                "mock",
                &mut bm25,
                &mut EmbedBreaker::default(),
            )
            .await
            .unwrap();
        let mut rows = lance_store.search_metadata("id <> ''", 10, Some("Default"), None).unwrap();
        let _ = std::fs::remove_dir_all(temp_dir);
        assert_eq!(rows.len(), 1);
        rows.remove(0)
    }

    #[tokio::test]
    async fn test_upsert_keeps_header_path_and_lines_with_envelope() {
        let row = upsert_one("envelope", chunk("md_0", "docs/guide.md", "guide.md > Timeouts", 40, 58, "markdown"), true).await;
        assert!(row.document_summary.as_deref().unwrap_or("").contains("guide.md"), "summary comes from the envelope");
        assert_eq!(row.header_path.as_deref(), Some("guide.md > Timeouts"));
        assert_eq!((row.start_line, row.end_line), (Some(40), Some(58)));

        let row = upsert_one("no_envelope", chunk("rs_0", "src/client.rs", "client.rs > fn connect", 12, 30, "rust"), false).await;
        assert!(row.document_summary.as_deref().unwrap_or("").is_empty(), "no envelope, no summary");
        assert_eq!(row.header_path.as_deref(), Some("client.rs > fn connect"));
        assert_eq!((row.start_line, row.end_line), (Some(12), Some(30)));
    }

    #[tokio::test]
    async fn test_upsert_skips_lines_for_page_relative_chunkers() {
        let row = upsert_one("pdf", chunk("pdf_0", "docs/manual.pdf", "manual.pdf > Page 3", 1, 42, "pdf"), true).await;
        assert_eq!(row.header_path.as_deref(), Some("manual.pdf > Page 3"));
        assert_eq!((row.start_line, row.end_line), (None, None));

        let row = upsert_one("web", chunk("web_0", "https://docs.example.com/guide", "Guide > Install", 5, 9, "markdown"), true).await;
        assert_eq!(row.header_path.as_deref(), Some("Guide > Install"));
        assert_eq!((row.start_line, row.end_line), (None, None));
    }
    /// What the scripted embedder answers on its next calls.
    #[derive(Clone, Copy)]
    enum EmbedBehavior {
        Ok,
        RateLimited,
        /// Rate-limited on the next call only, then `Ok`.
        RateLimitedOnce,
        Unauthorized,
        Short,
    }

    /// Embedding provider whose answer the test switches between passes.
    struct ScriptedEmbedder {
        behavior: std::sync::Mutex<EmbedBehavior>,
        calls: std::sync::atomic::AtomicUsize,
    }

    impl ScriptedEmbedder {
        fn new(behavior: EmbedBehavior) -> Arc<Self> {
            Arc::new(Self { behavior: std::sync::Mutex::new(behavior), calls: Default::default() })
        }

        fn set(&self, behavior: EmbedBehavior) {
            *self.behavior.lock().unwrap() = behavior;
        }
    }

    #[async_trait::async_trait]
    impl LmProvider for ScriptedEmbedder {
        fn provider_id(&self) -> &'static str {
            "scripted"
        }

        async fn chat_complete(&self, _request: actx_lm::types::ChatRequest) -> Result<actx_lm::types::ChatResponse, LmError> {
            Err(LmError::Unsupported("chat".into()))
        }

        async fn chat_stream(&self, _request: actx_lm::types::ChatRequest) -> Result<actx_lm::traits::BoxedChunkStream, LmError> {
            Err(LmError::Unsupported("chat".into()))
        }

        async fn embed(&self, _model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, LmError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut behavior = self.behavior.lock().unwrap();
            let rate_limited = LmError::RateLimit {
                provider: "scripted".into(),
                message: "quota exceeded".into(),
                retry_after_secs: Some(0),
            };
            match *behavior {
                EmbedBehavior::Ok => Ok(vec![vec![0.3; DEFAULT_VECTOR_DIM]; texts.len()]),
                EmbedBehavior::RateLimited => Err(rate_limited),
                EmbedBehavior::RateLimitedOnce => {
                    *behavior = EmbedBehavior::Ok;
                    Err(rate_limited)
                }
                EmbedBehavior::Unauthorized => Err(LmError::Auth { provider: "scripted".into(), message: "invalid key".into() }),
                EmbedBehavior::Short => Ok(vec![vec![0.3; DEFAULT_VECTOR_DIM]; texts.len().saturating_sub(1)]),
            }
        }
    }

    async fn upsert_with(tag: &str, provider: Arc<ScriptedEmbedder>, chunks: Vec<ChunkPayload>) -> (Result<usize, EmbedFailure>, usize, BM25Index) {
        let temp_dir = std::env::temp_dir().join(format!("test_orch_fail_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let db = Arc::new(NativeConfigDb::open_in_memory().unwrap());
        let lance_store = Arc::new(NativeLanceStore::open(&temp_dir.join("lancedb")).unwrap());
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db.clone());
        let enricher = NativeContextualEnricher::new(db);
        let mut bm25 = BM25Index::new(None, None);
        let file_path = chunks[0].file_path.clone();
        let file_name = chunks[0].file_name.clone();
        let lm: Option<Arc<dyn LmProvider>> = Some(provider);
        let result = orchestrator
            .embed_and_upsert_chunks(&chunks, None, &enricher, "Default", &file_path, &file_name, "failhash1234567", 1000.0, &lm, "scripted", &mut bm25, &mut EmbedBreaker::default())
            .await;
        let rows = lance_store.search_metadata("id <> ''", 100, Some("Default"), None).unwrap_or_default().len();
        let _ = std::fs::remove_dir_all(temp_dir);
        (result, rows, bm25)
    }

    #[tokio::test]
    async fn test_failed_embedding_writes_no_vectors_and_keeps_bm25() {
        let c = chunk("md_0", "docs/guide.md", "guide.md > Timeouts", 1, 5, "markdown");
        let (result, rows, bm25) = upsert_with("provider", ScriptedEmbedder::new(EmbedBehavior::RateLimited), vec![c]).await;
        assert!(matches!(result, Err(EmbedFailure::Provider(LmError::RateLimit { .. }))), "{result:?}");
        assert_eq!(rows, 0, "no pseudo-vectors may be stored for a failed call");
        assert_eq!(bm25.len(), 1, "the file stays reachable by keyword while it waits for a retry");
    }

    #[tokio::test]
    async fn test_short_embedding_answer_fails_the_file() {
        let chunks = vec![
            chunk("md_0", "docs/guide.md", "guide.md > A", 1, 5, "markdown"),
            chunk("md_1", "docs/guide.md", "guide.md > B", 6, 9, "markdown"),
        ];
        let (result, rows, _) = upsert_with("short", ScriptedEmbedder::new(EmbedBehavior::Short), chunks).await;
        assert!(matches!(result, Err(EmbedFailure::ShortAnswer { expected: 2, got: 1 })), "{result:?}");
        assert_eq!(rows, 0, "no zero vectors may fill the missing positions");
    }

    fn sync_fixture(tag: &str) -> (std::path::PathBuf, std::path::PathBuf, Arc<NativeConfigDb>, Arc<NativeLanceStore>) {
        let temp_dir = std::env::temp_dir().join(format!("test_orch_retry_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let docs_dir = temp_dir.join("docs");
        std::fs::create_dir_all(&docs_dir).unwrap();
        let db = Arc::new(NativeConfigDb::open_in_memory().unwrap());
        let lance_store = Arc::new(NativeLanceStore::open(&temp_dir.join("lancedb")).unwrap());
        (temp_dir, docs_dir, db, lance_store)
    }

    fn sync_opts(ws: &str, docs_dir: &Path) -> SyncOptions {
        SyncOptions {
            workspace: ws.to_string(),
            target_folder: Some(docs_dir.to_string_lossy().to_string()),
            model: Some("mock".to_string()),
            ..SyncOptions::default()
        }
    }

    #[tokio::test]
    async fn test_failed_file_is_retried_on_the_next_sync() {
        let (temp_dir, docs_dir, db, lance_store) = sync_fixture("next");
        let file = docs_dir.join("timeouts.md");
        std::fs::write(&file, "# Timeouts\n\nRequests time out after thirty seconds and are retried twice with backoff.").unwrap();
        let file_path = file.to_string_lossy().to_string();
        let embedder = ScriptedEmbedder::new(EmbedBehavior::RateLimited);
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db.clone()).with_embedding_provider(embedder.clone());
        let opts = sync_opts("RetryWS", &docs_dir);

        let first = orchestrator.run(&opts).await.unwrap();
        assert_eq!(first.indexed_files, 0);
        assert!(first.errors.iter().any(|e| e.contains("timeouts.md") && e.contains("embedding failed")), "{:?}", first.errors);
        assert_eq!(lance_store.count_records(Some("RetryWS"), None).unwrap_or(0), 0);
        let cached = db.get_workspace_files_stat_cache("RetryWS").unwrap_or_default();
        assert!(!cached.contains_key(&file_path), "a failed file must not be recorded as indexed");

        embedder.set(EmbedBehavior::Ok);
        let second = orchestrator.run(&opts).await.unwrap();
        assert!(second.errors.is_empty(), "{:?}", second.errors);
        assert_eq!(second.indexed_files, 1);
        let rows = lance_store.search_metadata("id <> ''", 100, Some("RetryWS"), None).unwrap();
        assert!(!rows.is_empty());

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_new_file_edited_before_retry_leaves_no_orphan_bm25_chunks() {
        let (temp_dir, docs_dir, db, lance_store) = sync_fixture("orphan");
        let file = docs_dir.join("ports.md");
        std::fs::write(&file, "# Ports\n\nThe service listens on port 8080 in the default configuration.").unwrap();
        let embedder = ScriptedEmbedder::new(EmbedBehavior::RateLimited);
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db).with_embedding_provider(embedder.clone());
        let opts = sync_opts("OrphanWS", &docs_dir);
        let bm25_path = lance_store.db_path().join("bm25_index.bin");
        let bm25_texts = || {
            BM25Index::load_from_file(bm25_path.to_str().unwrap())
                .map(|idx| idx.docs.into_iter().filter(|d| d.workspace == "OrphanWS").map(|d| d.text).collect::<Vec<_>>())
                .unwrap_or_default()
        };

        orchestrator.run(&opts).await.unwrap();
        assert!(bm25_texts().iter().any(|t| t.contains("8080")), "the failed file stays in BM25");

        std::fs::write(&file, "# Ports\n\nThe service now listens on port 9090 in the default configuration.").unwrap();
        embedder.set(EmbedBehavior::Ok);
        let second = orchestrator.run(&opts).await.unwrap();
        assert!(second.errors.is_empty(), "{:?}", second.errors);
        let texts = bm25_texts();
        assert!(texts.iter().any(|t| t.contains("9090")));
        assert!(texts.iter().all(|t| !t.contains("8080")), "orphan BM25 chunks of the failed version remain: {texts:?}");

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_failed_web_page_is_reported() {
        let (temp_dir, _docs_dir, db, lance_store) = sync_fixture("web");
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db.clone());
        let enricher = NativeContextualEnricher::new(db);
        let mut bm25 = BM25Index::new(None, None);
        let page = crate::ingestion::crawler::CrawledPage {
            url: "https://docs.example.com/limits".to_string(),
            title: "Limits".to_string(),
            text: "# Limits\n\nEach key may send sixty requests per minute before it is throttled.".to_string(),
            lastmod: None,
            etag: None,
            content_type: None,
        };
        let lm: Option<Arc<dyn LmProvider>> = Some(ScriptedEmbedder::new(EmbedBehavior::RateLimited));
        let err = orchestrator.index_web_page(&page, "WebWS", &enricher, &lm, "scripted", &mut bm25, &mut EmbedBreaker::default()).await.unwrap_err();
        assert!(err.contains("embedding failed"), "{err}");
        assert_eq!(lance_store.count_records(Some("WebWS"), None).unwrap_or(0), 0);

        let _ = std::fs::remove_dir_all(temp_dir);
    }
    fn calls(embedder: &ScriptedEmbedder) -> usize {
        embedder.calls.load(std::sync::atomic::Ordering::SeqCst)
    }

    #[tokio::test]
    async fn test_rate_limit_is_retried_once() {
        let embedder = ScriptedEmbedder::new(EmbedBehavior::RateLimitedOnce);
        let c = chunk("md_0", "docs/guide.md", "guide.md > Timeouts", 1, 5, "markdown");
        let (result, rows, _) = upsert_with("retry_once", embedder.clone(), vec![c]).await;
        assert_eq!(result.unwrap(), 1);
        assert_eq!(rows, 1);
        assert_eq!(calls(&embedder), 2);
    }

    #[tokio::test]
    async fn test_second_rate_limit_fails_the_file() {
        let embedder = ScriptedEmbedder::new(EmbedBehavior::RateLimited);
        let c = chunk("md_0", "docs/guide.md", "guide.md > Timeouts", 1, 5, "markdown");
        let (result, rows, _) = upsert_with("retry_twice", embedder.clone(), vec![c]).await;
        assert!(matches!(result, Err(EmbedFailure::Provider(LmError::RateLimit { .. }))), "{result:?}");
        assert_eq!(rows, 0);
        assert_eq!(calls(&embedder), 2, "a rate limit is retried exactly once");
    }

    fn write_docs(docs_dir: &Path, n: usize) {
        for i in 0..n {
            std::fs::write(
                docs_dir.join(format!("note_{i}.md")),
                format!("# Note {i}\n\nThe archive job number {i} runs every night and keeps thirty days of backups."),
            )
            .unwrap();
        }
    }

    fn bm25_files(lance_store: &NativeLanceStore, ws: &str) -> std::collections::HashSet<String> {
        let path = lance_store.db_path().join("bm25_index.bin");
        BM25Index::load_from_file(path.to_str().unwrap())
            .map(|idx| idx.docs.into_iter().filter(|d| d.workspace == ws).map(|d| d.file_path).collect())
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn test_auth_error_opens_the_breaker() {
        let (temp_dir, docs_dir, db, lance_store) = sync_fixture("auth");
        write_docs(&docs_dir, 4);
        let embedder = ScriptedEmbedder::new(EmbedBehavior::Unauthorized);
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db).with_embedding_provider(embedder.clone());

        let result = orchestrator.run(&sync_opts("AuthWS", &docs_dir)).await.unwrap();
        assert_eq!(calls(&embedder), 1, "a bad key must cost one call, not one per file");
        assert!(result.embedding_provider_unavailable, "the worker must not call the provider again for Global");
        assert_eq!(result.errors.len(), 4, "{:?}", result.errors);
        assert_eq!(result.errors.iter().filter(|e| e.contains("embedding skipped")).count(), 3, "{:?}", result.errors);
        assert_eq!(lance_store.count_records(Some("AuthWS"), None).unwrap_or(0), 0);
        assert_eq!(bm25_files(&lance_store, "AuthWS").len(), 4, "skipped files stay reachable by keyword");

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[tokio::test]
    async fn test_three_consecutive_failures_open_the_breaker() {
        let (temp_dir, docs_dir, db, lance_store) = sync_fixture("consecutive");
        write_docs(&docs_dir, 5);
        let embedder = ScriptedEmbedder::new(EmbedBehavior::RateLimited);
        let orchestrator = NativeSyncOrchestrator::new(lance_store.clone(), db).with_embedding_provider(embedder.clone());

        let result = orchestrator.run(&sync_opts("BreakerWS", &docs_dir)).await.unwrap();
        assert_eq!(calls(&embedder), 2 * BREAKER_THRESHOLD, "three files, each tried and retried once");
        assert!(result.embedding_provider_unavailable);
        assert_eq!(result.errors.len(), 5, "{:?}", result.errors);
        assert_eq!(result.errors.iter().filter(|e| e.contains("embedding skipped")).count(), 2, "{:?}", result.errors);
        assert_eq!(bm25_files(&lance_store, "BreakerWS").len(), 5);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_breaker_success_resets_the_count() {
        let mut breaker = EmbedBreaker::default();
        breaker.record_failure(false);
        breaker.record_failure(false);
        breaker.record_success();
        breaker.record_failure(false);
        breaker.record_failure(false);
        assert!(!breaker.is_open(), "failures separated by a success are not consecutive");
        breaker.record_failure(false);
        assert!(breaker.is_open());

        let mut breaker = EmbedBreaker::default();
        breaker.record_failure(true);
        assert!(breaker.is_open(), "an authentication error opens the breaker at once");
    }
    #[tokio::test]
    async fn test_sync_status_error_summary_set_and_cleared() {
        let (temp_dir, docs_dir, db, lance_store) = sync_fixture("status");
        write_docs(&docs_dir, 2);
        let embedder = ScriptedEmbedder::new(EmbedBehavior::RateLimited);
        let orchestrator = NativeSyncOrchestrator::new(lance_store, db.clone()).with_embedding_provider(embedder.clone());
        let opts = sync_opts("StatusWS", &docs_dir);

        let first = orchestrator.run(&opts).await.unwrap();
        assert_eq!(first.embedding_failures, 2);
        let error = db.get_sync_status("StatusWS").unwrap().unwrap().error.expect("a failed pass records a summary");
        assert_eq!(sync_error_headline(&error), "2 files not embedded");
        assert!(error.contains("they are retried on the next sync"), "{error}");
        assert!(error.contains("quota exceeded"), "the first reason is kept: {error}");

        embedder.set(EmbedBehavior::Ok);
        let second = orchestrator.run(&opts).await.unwrap();
        assert_eq!(second.embedding_failures, 0);
        assert!(!second.embedding_provider_unavailable);
        assert_eq!(db.get_sync_status("StatusWS").unwrap().unwrap().error, None, "a clean pass clears the summary");

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_embedding_failure_summary_headline() {
        assert_eq!(sync_error_headline(&embedding_failure_summary(1, "x; y")), "1 file not embedded");
        assert_eq!(sync_error_headline("plain error"), "plain error");
    }
}
