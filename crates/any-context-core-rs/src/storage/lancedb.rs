//! Native LanceDB Vector Storage Driver for AnyContext.
//!
//! Provides thread-safe Apache Arrow columnar vector persistence in Rust,
//! delivering zero-copy sub-millisecond similarity search and metadata filtering.

use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray,
    UInt32Array,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::optimize::Duration;
use lancedb::table::{CompactionOptions, NewColumnTransform, OptimizeAction, Table};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const DEFAULT_TABLE_NAME: &str = "workspace_chunks";
pub const DEFAULT_VECTOR_DIM: usize = 1536;

/// Rows with a protected field still in plaintext (written before every write was encrypted).
const PLAINTEXT_ROWS_FILTER: &str = "(text <> '' AND text NOT LIKE 'enc::%') \
    OR (document_summary IS NOT NULL AND document_summary <> '' AND document_summary NOT LIKE 'enc::%') \
    OR (keywords IS NOT NULL AND keywords <> '' AND keywords NOT LIKE 'enc::%') \
    OR (header_path IS NOT NULL AND header_path <> '' AND header_path NOT LIKE 'enc::%')";
const ENCRYPTION_MIGRATION_BATCH_ROWS: usize = 1000;

/// Outcome of `NativeLanceStore::encrypt_plaintext_rows`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EncryptionMigrationReport {
    /// Rows rewritten with their protected fields encrypted.
    pub rows_encrypted: usize,
    /// Old dataset versions (which held the plaintext) were removed.
    pub versions_pruned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: String,
    pub vector: Vec<f32>,
    pub text: String,
    pub file_name: String,
    pub file_path: String,
    pub workspace: String,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
    pub document_summary: Option<String>,
    pub keywords: Option<String>,
    pub content_hash: Option<String>,
    /// Section or symbol path of the chunk (encrypted at rest).
    #[serde(default)]
    pub header_path: Option<String>,
    /// First and last file line of the chunk, only for chunkers that emit file line numbers.
    #[serde(default)]
    pub start_line: Option<u32>,
    #[serde(default)]
    pub end_line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredVectorResult {
    pub id: String,
    pub text: String,
    pub file_name: String,
    pub file_path: String,
    pub workspace: String,
    pub score: f32,
    pub distance: f32,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
    pub document_summary: Option<String>,
    pub keywords: Option<String>,
    pub content_hash: Option<String>,
    /// Section or symbol path of the chunk (encrypted at rest).
    #[serde(default)]
    pub header_path: Option<String>,
    /// First and last file line of the chunk, only for chunkers that emit file line numbers.
    #[serde(default)]
    pub start_line: Option<u32>,
    #[serde(default)]
    pub end_line: Option<u32>,
}

pub struct NativeLanceStore {
    db_path: PathBuf,
    runtime: Option<Arc<tokio::runtime::Runtime>>,
    conn: Connection,
}

impl Drop for NativeLanceStore {
    fn drop(&mut self) {
        if let Some(rt) = self.runtime.take() {
            if tokio::runtime::Handle::try_current().is_ok() {
                let _ = std::thread::spawn(move || {
                    drop(rt);
                }).join();
            } else {
                drop(rt);
            }
        }
    }
}

/// Returns the single canonical path for AnyContext LanceDB vector storage.
/// - Windows: %LOCALAPPDATA%\AnyContext\data\context_db\lancedb
/// - macOS: ~/Library/Application Support/AnyContext/data/context_db/lancedb
/// - Linux: ~/.local/share/any-context/data/context_db/lancedb
/// Can be overridden via ACTX_CONTEXT_DB environment variable.
pub fn get_default_lancedb_path() -> PathBuf {
    if let Ok(p) = std::env::var("ACTX_CONTEXT_DB") {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            let pb = PathBuf::from(trimmed);
            if pb.ends_with("lancedb") {
                return pb;
            } else {
                return pb.join("lancedb");
            }
        }
    }

    if std::env::var("ACTX_TEST_MODE").is_ok() {
        let sandbox = std::env::temp_dir().join("actx_test_sandbox").join("data").join("context_db").join("lancedb");
        let _ = std::fs::create_dir_all(&sandbox);
        return sandbox;
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("AnyContext").join("data").join("context_db").join("lancedb");
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("AnyContext").join("data").join("context_db").join("lancedb");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join("Library").join("Application Support").join("AnyContext").join("data").join("context_db").join("lancedb");
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("any-context").join("data").join("context_db").join("lancedb");
        }
        if let Some(home) = dirs::home_dir() {
            return home.join(".local").join("share").join("any-context").join("data").join("context_db").join("lancedb");
        }
    }

    if let Some(data_dir) = dirs::data_local_dir() {
        return data_dir.join("AnyContext").join("data").join("context_db").join("lancedb");
    }

    PathBuf::from("./data/context_db/lancedb")
}

impl NativeLanceStore {
    /// Opens the LanceDB database at the canonical system default location.
    pub fn open_default() -> Result<Self, String> {
        Self::open(get_default_lancedb_path())
    }

    /// Opens or creates a LanceDB database directory.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let p = db_path.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&p);
        let uri = p.to_string_lossy().to_string();

        let (conn, rt) = if tokio::runtime::Handle::try_current().is_ok() {
            let uri_clone = uri.clone();
            let conn = std::thread::spawn(move || {
                let temp_rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?;
                temp_rt
                    .block_on(async { lancedb::connect(&uri_clone).execute().await })
                    .map_err(|e| format!("Failed to connect to LanceDB at {uri_clone}: {e}"))
            })
            .join()
            .map_err(|_| "LanceDB connection thread panicked".to_string())??;
            (conn, None)
        } else {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|e| format!("Failed to initialize Tokio runtime for LanceDB: {e}"))?;
            let conn = rt.block_on(async { lancedb::connect(&uri).execute().await })
                .map_err(|e| format!("Failed to connect to LanceDB at {uri}: {e}"))?;
            (conn, Some(Arc::new(rt)))
        };

        Ok(Self {
            db_path: p,
            runtime: rt,
            conn,
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn runtime(&self) -> Option<&Arc<tokio::runtime::Runtime>> {
        self.runtime.as_ref()
    }


    pub fn get_schema(dim: usize) -> Arc<Schema> {
        let mut fields = vec![
            Field::new("id", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dim as i32,
                ),
                false,
            ),
            Field::new("text", DataType::Utf8, false),
            Field::new("file_name", DataType::Utf8, false),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("workspace", DataType::Utf8, false),
            Field::new("last_modified", DataType::Utf8, true),
            Field::new("content_type", DataType::Utf8, true),
            Field::new("document_summary", DataType::Utf8, true),
            Field::new("keywords", DataType::Utf8, true),
            Field::new("content_hash", DataType::Utf8, true),
        ];
        fields.extend(Self::provenance_fields());
        Arc::new(Schema::new(fields))
    }

    /// Nullable provenance columns added after the first table schema (ADR-0142).
    fn provenance_fields() -> Vec<Field> {
        vec![
            Field::new("header_path", DataType::Utf8, true),
            Field::new("start_line", DataType::UInt32, true),
            Field::new("end_line", DataType::UInt32, true),
        ]
    }

    /// Adds the provenance columns that a table created by an older version lacks, as all-null
    /// columns: a schema-only commit, no row is rewritten or re-embedded. Returns whether
    /// columns were added. With the columns present it only reads the schema.
    async fn add_missing_provenance_columns(table: &Table) -> Result<bool, String> {
        let missing = Self::missing_provenance_fields(table).await?;
        if missing.is_empty() {
            return Ok(false);
        }
        let added = table
            .add_columns()
            .transform(NewColumnTransform::AllNulls(Arc::new(Schema::new(missing))))
            .execute()
            .await;
        match added {
            Ok(_) => Ok(true),
            // Another process (TUI and sync worker share the table) may have added them first.
            Err(e) => {
                let _ = table.checkout_latest().await;
                if Self::missing_provenance_fields(table).await?.is_empty() {
                    Ok(false)
                } else {
                    Err(format!("Adding provenance columns to LanceDB failed: {e}"))
                }
            }
        }
    }

    async fn missing_provenance_fields(table: &Table) -> Result<Vec<Field>, String> {
        let schema = table.schema().await.map_err(|e| e.to_string())?;
        Ok(Self::provenance_fields()
            .into_iter()
            .filter(|f| schema.field_with_name(f.name()).is_err())
            .collect())
    }

    pub async fn table_exists_async(&self, table_name: &str) -> Result<bool, String> {
        let names = self
            .conn
            .table_names()
            .execute()
            .await
            .map_err(|e| e.to_string())?;
        Ok(names.contains(&table_name.to_string()))
    }

    /// Safely executes an asynchronous future synchronously without risking
    /// "Cannot start a runtime from within a runtime" panics if called from inside a Tokio worker thread.
    fn safe_block_on<F, T>(&self, future: F) -> T
    where
        F: std::future::Future<Output = T> + Send,
        T: Send,
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            std::thread::scope(|s| {
                s.spawn(|| {
                    let temp_rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("Failed to initialize temporary worker runtime");
                    temp_rt.block_on(future)
                })
                .join()
                .expect("LanceDB worker thread panicked")
            })
        } else if let Some(ref rt) = self.runtime {
            rt.block_on(future)
        } else {
            let temp_rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to initialize temporary worker runtime");
            temp_rt.block_on(future)
        }
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool, String> {
        self.safe_block_on(self.table_exists_async(table_name))
    }

    /// Migrates a table created by an older version to the provenance columns (ADR-0142).
    /// Returns whether columns were added; a missing table is a no-op. Writes also run it.
    pub fn ensure_provenance_columns(&self, table_name: Option<&str>) -> Result<bool, String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(false);
        }
        self.safe_block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;
            Self::add_missing_provenance_columns(&table).await
        })
    }

    pub fn get_or_create_table(&self, table_name: &str, dim: usize) -> Result<Table, String> {
        self.safe_block_on(async {

            let names = self
                .conn
                .table_names()
                .execute()
                .await
                .map_err(|e| e.to_string())?;
            if names.contains(&table_name.to_string()) {
                let table = self
                    .conn
                    .open_table(table_name)
                    .execute()
                    .await
                    .map_err(|e| e.to_string())?;
                Self::add_missing_provenance_columns(&table).await?;
                Ok(table)
            } else {
                let schema = Self::get_schema(dim);
                self.conn
                    .create_empty_table(table_name, schema)
                    .execute()
                    .await
                    .map_err(|e| e.to_string())
            }
        })
    }

    /// Inserts vector records into LanceDB in an Apache Arrow columnar batch.
    pub fn upsert_records(
        &self,
        records: Vec<VectorRecord>,
        table_name: Option<&str>,
        dim: Option<usize>,
    ) -> Result<usize, String> {
        if records.is_empty() {
            return Ok(0);
        }

        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        let d = dim.unwrap_or(DEFAULT_VECTOR_DIM);
        let num_records = records.len();
        let records = records
            .into_iter()
            .map(|mut r| {
                r.file_path = r.file_path.replace('\\', "/");
                r
            })
            .collect();
        let (batch, schema) = Self::build_record_batch(records, d)?;

        let table = self.get_or_create_table(tname, d)?;

        self.safe_block_on(async {
            let batch_iter = RecordBatchIterator::new(vec![Ok(batch)], schema);
            let reader: Box<dyn arrow_array::RecordBatchReader + Send> = Box::new(batch_iter);
            table
                .add(reader)
                .execute()
                .await
                .map_err(|e| format!("Failed to insert records into LanceDB: {e}"))?;
            Ok(num_records)
        })
    }

    /// Builds the Arrow batch written to LanceDB, encrypting the protected fields.
    fn build_record_batch(
        records: Vec<VectorRecord>,
        d: usize,
    ) -> Result<(RecordBatch, Arc<Schema>), String> {
        let num_records = records.len();
        let schema = Self::get_schema(d);

        // Build Arrow Arrays column by column
        let mut ids = Vec::with_capacity(num_records);
        let mut flat_vectors = Vec::with_capacity(num_records * d);
        let mut texts = Vec::with_capacity(num_records);
        let mut file_names = Vec::with_capacity(num_records);
        let mut file_paths = Vec::with_capacity(num_records);
        let mut workspaces = Vec::with_capacity(num_records);
        let mut last_modifieds = Vec::with_capacity(num_records);
        let mut content_types = Vec::with_capacity(num_records);
        let mut doc_summaries = Vec::with_capacity(num_records);
        let mut keywords_list = Vec::with_capacity(num_records);
        let mut hashes = Vec::with_capacity(num_records);
        let mut header_paths = Vec::with_capacity(num_records);
        let mut start_lines = Vec::with_capacity(num_records);
        let mut end_lines = Vec::with_capacity(num_records);

        // Field-level encryption at rest: `text`, `document_summary`, `keywords` and `header_path` are stored as
        // AES-GCM-256 `enc::` payloads with the hardware-bound key; `extract_scored_results`
        // decrypts them on read. Ids, vectors and filterable metadata stay in clear text.
        let sec = crate::security::NativeSecurityEngine::get_instance();

        for r in records {
            // Shorter vectors are zero-padded (queries are padded the same way in
            // `fit_query_vector`); longer ones can't be truncated without loss.
            if r.vector.len() > d {
                return Err(format!(
                    "chunk '{}' has a {}-value vector, but the chunk table holds {d}-value vectors",
                    r.id,
                    r.vector.len()
                ));
            }
            ids.push(r.id);
            let mut padded = r.vector;
            padded.resize(d, 0.0);
            flat_vectors.extend(padded);
            texts.push(sec.encrypt_text(&r.text));
            file_names.push(r.file_name);
            file_paths.push(r.file_path);
            workspaces.push(r.workspace);
            last_modifieds.push(r.last_modified.unwrap_or_default());
            content_types.push(r.content_type.unwrap_or_else(|| "Local Document".into()));
            doc_summaries.push(sec.encrypt_text(&r.document_summary.unwrap_or_default()));
            keywords_list.push(sec.encrypt_text(&r.keywords.unwrap_or_default()));
            hashes.push(r.content_hash.unwrap_or_default());
            header_paths.push(r.header_path.map(|h| sec.encrypt_text(&h)));
            start_lines.push(r.start_line);
            end_lines.push(r.end_line);
        }

        let id_arr = Arc::new(StringArray::from(ids));
        let values_arr = Arc::new(Float32Array::from(flat_vectors));
        let vector_arr = Arc::new(FixedSizeListArray::new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            d as i32,
            values_arr,
            None,
        ));
        let text_arr = Arc::new(StringArray::from(texts));
        let fn_arr = Arc::new(StringArray::from(file_names));
        let fp_arr = Arc::new(StringArray::from(file_paths));
        let ws_arr = Arc::new(StringArray::from(workspaces));
        let lm_arr = Arc::new(StringArray::from(last_modifieds));
        let ct_arr = Arc::new(StringArray::from(content_types));
        let ds_arr = Arc::new(StringArray::from(doc_summaries));
        let kw_arr = Arc::new(StringArray::from(keywords_list));
        let h_arr = Arc::new(StringArray::from(hashes));
        let hp_arr = Arc::new(StringArray::from(header_paths));
        let sl_arr = Arc::new(UInt32Array::from(start_lines));
        let el_arr = Arc::new(UInt32Array::from(end_lines));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                id_arr,
                vector_arr,
                text_arr,
                fn_arr,
                fp_arr,
                ws_arr,
                lm_arr,
                ct_arr,
                ds_arr,
                kw_arr,
                h_arr,
                hp_arr,
                sl_arr,
                el_arr,
            ],
        )
        .map_err(|e| format!("Failed to create Arrow RecordBatch: {e}"))?;

        Ok((batch, schema))
    }

    /// Encrypts the rows that still hold `text`, `document_summary` or `keywords` in plaintext
    /// (written before every write was encrypted) and removes the old dataset files that kept
    /// the plaintext on disk. Rows are rewritten in place with their vectors; nothing is
    /// re-embedded. Idempotent: with no plaintext row left it only runs one count.
    pub fn encrypt_plaintext_rows(
        &self,
        table_name: Option<&str>,
    ) -> Result<EncryptionMigrationReport, String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(EncryptionMigrationReport::default());
        }

        self.safe_block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;
            // The plaintext filter and the rewritten rows use the provenance columns.
            Self::add_missing_provenance_columns(&table).await?;

            let mut remaining = Self::count_plaintext_rows(&table).await?;
            if remaining == 0 {
                return Ok(EncryptionMigrationReport::default());
            }
            let dim = Self::table_vector_dim(&table).await?;
            let mut report = EncryptionMigrationReport::default();

            while remaining > 0 {
                let mut stream = table
                    .query()
                    .only_if(PLAINTEXT_ROWS_FILTER)
                    .limit(ENCRYPTION_MIGRATION_BATCH_ROWS)
                    .execute()
                    .await
                    .map_err(|e| format!("Reading plaintext rows failed: {e}"))?;
                let mut records = Vec::new();
                while let Some(batch) = stream.try_next().await.map_err(|e| e.to_string())? {
                    records.extend(Self::extract_vector_records(&batch)?);
                }

                // Chunk ids repeat across paths and workspaces, so rows are matched on all three
                // columns; a duplicate source key would make the merge ambiguous.
                let mut seen = std::collections::HashSet::new();
                records.retain(|r| seen.insert((r.id.clone(), r.workspace.clone(), r.file_path.clone())));

                let (batch, schema) = Self::build_record_batch(records, dim)?;
                let reader: Box<dyn arrow_array::RecordBatchReader + Send> =
                    Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));
                let mut merge = table.merge_insert(&["id", "workspace", "file_path"]);
                merge.when_matched_update_all(None);
                let merged = merge
                    .execute(reader)
                    .await
                    .map_err(|e| format!("Encrypting plaintext rows failed: {e}"))?;
                report.rows_encrypted += merged.num_updated_rows as usize;

                let left = Self::count_plaintext_rows(&table).await?;
                if left >= remaining {
                    return Err(format!(
                        "Encrypting plaintext rows made no progress ({left} rows still in plaintext)"
                    ));
                }
                remaining = left;
            }

            // Updated rows only get deletion markers in their old fragments, and older versions
            // keep referencing those files: compact every fragment with deletions, then drop
            // every version but the current one so the plaintext leaves the disk.
            table
                .optimize(OptimizeAction::Compact {
                    options: CompactionOptions {
                        materialize_deletions_threshold: 0.0,
                        ..CompactionOptions::default()
                    },
                    remap_options: None,
                })
                .await
                .map_err(|e| format!("Compacting the encrypted table failed: {e}"))?;
            table
                .optimize(OptimizeAction::Prune {
                    older_than: Some(Duration::zero()),
                    delete_unverified: Some(false),
                    error_if_tagged_old_versions: Some(false),
                })
                .await
                .map_err(|e| format!("Pruning plaintext table versions failed: {e}"))?;
            report.versions_pruned = true;

            Ok(report)
        })
    }

    async fn count_plaintext_rows(table: &Table) -> Result<usize, String> {
        table
            .count_rows(Some(PLAINTEXT_ROWS_FILTER.to_string()))
            .await
            .map_err(|e| format!("Counting plaintext rows failed: {e}"))
    }

    async fn table_vector_dim(table: &Table) -> Result<usize, String> {
        let schema = table.schema().await.map_err(|e| e.to_string())?;
        match schema.field_with_name("vector").map(|f| f.data_type().clone()) {
            Ok(DataType::FixedSizeList(_, dim)) => Ok(dim as usize),
            _ => Err("Table has no fixed-size 'vector' column".to_string()),
        }
    }

    /// Zero-pads a shorter query vector to the table dimension, as `build_record_batch` pads the
    /// stored vectors, so both live in the same space: the padded positions add nothing to L2,
    /// cosine or dot-product scores (BUG-021). A longer vector can't be truncated without loss.
    fn fit_query_vector(mut query_vector: Vec<f32>, dim: usize) -> Result<Vec<f32>, String> {
        if query_vector.len() > dim {
            return Err(format!(
                "query vector has {} values, but the chunk table holds {dim}-value vectors",
                query_vector.len()
            ));
        }
        query_vector.resize(dim, 0.0);
        Ok(query_vector)
    }

    /// Performs vector similarity search with optional workspace filtering asynchronously.
    pub async fn search_vector_async(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        workspace: Option<&str>,
        filter_expr: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<Vec<ScoredVectorResult>, String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists_async(tname).await? {
            return Ok(Vec::new());
        }

        let table = self
            .conn
            .open_table(tname)
            .execute()
            .await
            .map_err(|e| e.to_string())?;

        let dim = Self::table_vector_dim(&table).await?;
        let query_vector = Self::fit_query_vector(query_vector, dim)?;
        let mut query = table.vector_search(query_vector).map_err(|e| e.to_string())?;
        query = query.limit(limit);

        let mut where_clauses = Vec::new();
        if let Some(ws) = workspace {
            let clean_ws = ws.replace('\'', "''");
            where_clauses.push(format!("(workspace = '{clean_ws}' OR workspace = 'Global')"));
        }
        if let Some(f) = filter_expr {
            where_clauses.push(f.to_string());
        }

        if !where_clauses.is_empty() {
            query = query.only_if(where_clauses.join(" AND "));
        }

        let mut stream = query.execute().await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();

        while let Some(batch) = stream.try_next().await.map_err(|e| e.to_string())? {
            results.extend(Self::extract_scored_results(&batch)?);
        }

        Ok(results)
    }

    /// Performs vector similarity search with optional workspace filtering synchronously.
    pub fn search_vector(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        workspace: Option<&str>,
        filter_expr: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<Vec<ScoredVectorResult>, String> {
        self.safe_block_on(self.search_vector_async(
            query_vector,
            limit,
            workspace,
            filter_expr,
            table_name,
        ))
    }

    /// Performs ultra-fast (<25ms) metadata filtering on LanceDB without computing embeddings asynchronously.
    pub async fn search_metadata_async(
        &self,
        where_clause: &str,
        limit: usize,
        workspace: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<Vec<ScoredVectorResult>, String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists_async(tname).await? || where_clause.is_empty() {
            return Ok(Vec::new());
        }

        let table = self
            .conn
            .open_table(tname)
            .execute()
            .await
            .map_err(|e| e.to_string())?;

        let mut where_clauses = Vec::new();
        if let Some(ws) = workspace {
            let clean_ws = ws.replace('\'', "''");
            where_clauses.push(format!("(workspace = '{clean_ws}' OR workspace = 'Global')"));
        }
        where_clauses.push(format!("({where_clause})"));

        let combined_filter = where_clauses.join(" AND ");
        let query = table.query().only_if(combined_filter).limit(limit);

        let mut stream = query.execute().await.map_err(|e| e.to_string())?;
        let mut results = Vec::new();

        while let Some(batch) = stream.try_next().await.map_err(|e| e.to_string())? {
            results.extend(Self::extract_scored_results(&batch)?);
        }

        Ok(results)
    }

    /// Performs ultra-fast (<25ms) metadata filtering on LanceDB without computing embeddings synchronously.
    pub fn search_metadata(
        &self,
        where_clause: &str,
        limit: usize,
        workspace: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<Vec<ScoredVectorResult>, String> {
        self.safe_block_on(self.search_metadata_async(
            where_clause,
            limit,
            workspace,
            table_name,
        ))
    }

    /// Counts rows in the table or scoped to a specific workspace.
    pub fn count_records(
        &self,
        workspace: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<usize, String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(0);
        }

        self.safe_block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;

            let filter = workspace.map(|ws| {
                let clean_ws = ws.replace('\'', "''");
                format!("workspace = '{clean_ws}'")
            });

            table
                .count_rows(filter)
                .await
                .map_err(|e| format!("Count failed: {e}"))
        })
    }

    pub fn delete_by_workspace(
        &self,
        workspace: &str,
        table_name: Option<&str>,
    ) -> Result<(), String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(());
        }

        self.safe_block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;
            let clean_ws = workspace.replace('\'', "''");
            let pred = format!("workspace = '{clean_ws}'");
            table
                .delete(pred.as_str())
                .await
                .map_err(|e| format!("Delete by workspace failed: {e}"))?;
            Ok(())
        })
    }

    pub fn delete_by_file(
        &self,
        file_path: &str,
        workspace: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<(), String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(());
        }

        self.safe_block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;

            let clean_fp = file_path.replace('\\', "/").replace('\'', "''");
            let clean_dir = clean_fp.trim_end_matches('/');
            let mut clause = format!("(file_path = '{clean_fp}' OR file_path LIKE '{clean_dir}/%')");
            if let Some(ws) = workspace {
                let clean_ws = ws.replace('\'', "''");
                clause = format!("workspace = '{clean_ws}' AND ({clause})");
            }

            table
                .delete(clause.as_str())
                .await
                .map_err(|e| format!("Delete by file failed: {e}"))?;
            Ok(())
        })
    }

    /// Deletes the chunks whose `file_path` is exactly `file_path` (no folder prefix match).
    /// Used for web pages, where one URL can be a path prefix of another page's URL.
    pub fn delete_by_exact_path(
        &self,
        file_path: &str,
        workspace: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<(), String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(());
        }

        self.safe_block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;

            let clean_fp = file_path.replace('\\', "/").replace('\'', "''");
            let mut clause = format!("file_path = '{clean_fp}'");
            if let Some(ws) = workspace {
                let clean_ws = ws.replace('\'', "''");
                clause = format!("workspace = '{clean_ws}' AND {clause}");
            }

            table
                .delete(clause.as_str())
                .await
                .map_err(|e| format!("Delete by exact path failed: {e}"))?;
            Ok(())
        })
    }

    pub fn delete_by_id(&self, chunk_id: &str, table_name: Option<&str>) -> Result<(), String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(());
        }

        self.safe_block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;
            let clean_id = chunk_id.replace('\'', "''");
            let pred = format!("id = '{clean_id}'");
            table
                .delete(pred.as_str())
                .await
                .map_err(|e| format!("Delete by id failed: {e}"))?;
            Ok(())
        })
    }

    /// Reads the provenance columns of row `i`; tables not migrated yet have none (all `None`).
    /// `header_path` is returned as stored (encrypted).
    fn provenance_at(batch: &RecordBatch, i: usize) -> (Option<String>, Option<u32>, Option<u32>) {
        let header = batch
            .column_by_name("header_path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>())
            .and_then(|c| (!c.is_null(i)).then(|| c.value(i).to_string()));
        let line = |name: &str| {
            batch
                .column_by_name(name)
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>())
                .and_then(|c| (!c.is_null(i)).then(|| c.value(i)))
        };
        (header, line("start_line"), line("end_line"))
    }

    /// Reads full rows (vector included) with the stored values as they are, without decrypting.
    fn extract_vector_records(batch: &RecordBatch) -> Result<Vec<VectorRecord>, String> {
        let string_col = |name: &str| -> Result<&StringArray, String> {
            batch
                .column_by_name(name)
                .ok_or(format!("Missing '{name}' column"))?
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(format!("Failed downcast '{name}'"))
        };
        let optional = |col: &StringArray, i: usize| -> Option<String> {
            (!col.is_null(i)).then(|| col.value(i).to_string())
        };

        let vector_col = batch
            .column_by_name("vector")
            .ok_or("Missing 'vector' column")?
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .ok_or("Failed downcast 'vector'")?;
        let (id, text, file_name, file_path, workspace) = (
            string_col("id")?,
            string_col("text")?,
            string_col("file_name")?,
            string_col("file_path")?,
            string_col("workspace")?,
        );
        let (last_modified, content_type, summary, keywords, hash) = (
            string_col("last_modified")?,
            string_col("content_type")?,
            string_col("document_summary")?,
            string_col("keywords")?,
            string_col("content_hash")?,
        );

        let mut records = Vec::with_capacity(batch.num_rows());
        for i in 0..batch.num_rows() {
            let values = vector_col.value(i);
            let vector = values
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or("Failed downcast 'vector' values")?
                .values()
                .to_vec();
            let (header_path, start_line, end_line) = Self::provenance_at(batch, i);
            records.push(VectorRecord {
                id: id.value(i).to_string(),
                vector,
                text: text.value(i).to_string(),
                file_name: file_name.value(i).to_string(),
                file_path: file_path.value(i).to_string(),
                workspace: workspace.value(i).to_string(),
                last_modified: optional(last_modified, i),
                content_type: optional(content_type, i),
                document_summary: optional(summary, i),
                keywords: optional(keywords, i),
                content_hash: optional(hash, i),
                header_path,
                start_line,
                end_line,
            });
        }
        Ok(records)
    }

    fn extract_scored_results(batch: &RecordBatch) -> Result<Vec<ScoredVectorResult>, String> {
        let num_rows = batch.num_rows();
        let mut results = Vec::with_capacity(num_rows);

        let id_col = batch
            .column_by_name("id")
            .ok_or("Missing 'id' column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Failed downcast 'id'")?;

        let text_col = batch
            .column_by_name("text")
            .ok_or("Missing 'text' column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Failed downcast 'text'")?;

        let fn_col = batch
            .column_by_name("file_name")
            .ok_or("Missing 'file_name' column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Failed downcast 'file_name'")?;

        let fp_col = batch
            .column_by_name("file_path")
            .ok_or("Missing 'file_path' column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Failed downcast 'file_path'")?;

        let ws_col = batch
            .column_by_name("workspace")
            .ok_or("Missing 'workspace' column")?
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or("Failed downcast 'workspace'")?;

        let dist_col = batch
            .column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

        let lm_col = batch
            .column_by_name("last_modified")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        let ct_col = batch
            .column_by_name("content_type")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        let ds_col = batch
            .column_by_name("document_summary")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        let kw_col = batch
            .column_by_name("keywords")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        let h_col = batch
            .column_by_name("content_hash")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

        let sec = crate::security::NativeSecurityEngine::get_instance();

        for i in 0..num_rows {
            let dist = dist_col.map(|d| d.value(i)).unwrap_or(0.0);
            let score = 1.0 / (1.0 + dist.max(0.0));

            let raw_text = text_col.value(i);
            let raw_ds = ds_col.map(|c| c.value(i));
            let raw_kw = kw_col.map(|c| c.value(i));

            let text = sec.decrypt_text(raw_text);
            let document_summary = raw_ds.map(|s| sec.decrypt_text(s));
            let keywords = raw_kw.map(|s| sec.decrypt_text(s));
            let (raw_header, start_line, end_line) = Self::provenance_at(batch, i);
            let header_path = raw_header.map(|h| sec.decrypt_text(&h)).filter(|h| !h.is_empty());

            results.push(ScoredVectorResult {
                id: id_col.value(i).to_string(),
                text,
                file_name: fn_col.value(i).to_string(),
                file_path: fp_col.value(i).to_string(),
                workspace: ws_col.value(i).to_string(),
                score,
                distance: dist,
                last_modified: lm_col.map(|c| c.value(i).to_string()),
                content_type: ct_col.map(|c| c.value(i).to_string()),
                document_summary,
                keywords,
                content_hash: h_col.map(|c| c.value(i).to_string()),
                header_path,
                start_line,
                end_line,
            });
        }

        Ok(results)
    }

    pub fn get_db_path(&self) -> &Path {
        &self.db_path
    }
}

#[cfg(test)]
impl NativeLanceStore {
    /// Writes rows the way versions before ADR-0131 did: protected fields in plaintext.
    pub(crate) fn write_plaintext_rows_for_test(
        &self,
        table_name: Option<&str>,
        d: usize,
        records: Vec<VectorRecord>,
    ) {
        let (batch, schema) = Self::plaintext_batch_for_test(d, &records);
        let table = self
            .get_or_create_table(table_name.unwrap_or(DEFAULT_TABLE_NAME), d)
            .expect("table");
        self.safe_block_on(async {
            let reader: Box<dyn arrow_array::RecordBatchReader + Send> =
                Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));
            table.add(reader).execute().await.expect("raw add");
        });
    }

    /// Creates a table with the schema of versions before ADR-0142 (no provenance columns)
    /// and writes `records` into it in plaintext.
    pub(crate) fn create_legacy_table_for_test(&self, table_name: &str, d: usize, records: Vec<VectorRecord>) {
        let (batch, _) = Self::plaintext_batch_for_test(d, &records);
        let legacy_columns: Vec<usize> = (0..batch.num_columns() - Self::provenance_fields().len()).collect();
        let batch = batch.project(&legacy_columns).expect("legacy projection");
        let schema = batch.schema();
        self.safe_block_on(async {
            let table = self
                .conn
                .create_empty_table(table_name, schema.clone())
                .execute()
                .await
                .expect("legacy table");
            let reader: Box<dyn arrow_array::RecordBatchReader + Send> =
                Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));
            table.add(reader).execute().await.expect("legacy add");
        });
    }

    fn plaintext_batch_for_test(d: usize, records: &[VectorRecord]) -> (RecordBatch, Arc<Schema>) {
        let schema = Self::get_schema(d);
        let col = |f: &dyn Fn(&VectorRecord) -> Option<String>| -> Arc<StringArray> {
            Arc::new(StringArray::from(records.iter().map(f).collect::<Vec<_>>()))
        };
        let vectors: Vec<f32> = records.iter().flat_map(|r| r.vector.clone()).collect();
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                col(&|r| Some(r.id.clone())),
                Arc::new(FixedSizeListArray::new(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    d as i32,
                    Arc::new(Float32Array::from(vectors)),
                    None,
                )),
                col(&|r| Some(r.text.clone())),
                col(&|r| Some(r.file_name.clone())),
                col(&|r| Some(r.file_path.clone())),
                col(&|r| Some(r.workspace.clone())),
                col(&|r| r.last_modified.clone()),
                col(&|r| r.content_type.clone()),
                col(&|r| r.document_summary.clone()),
                col(&|r| r.keywords.clone()),
                col(&|r| r.content_hash.clone()),
                col(&|r| r.header_path.clone()),
                Arc::new(UInt32Array::from(records.iter().map(|r| r.start_line).collect::<Vec<_>>())),
                Arc::new(UInt32Array::from(records.iter().map(|r| r.end_line).collect::<Vec<_>>())),
            ],
        )
        .expect("plaintext batch");
        (batch, schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lance_create_table_upsert_and_search() {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_lance_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let store = NativeLanceStore::open(&temp_dir).expect("open lance store");

        let records = vec![
            VectorRecord {
                id: "c1".into(),
                vector: vec![0.1; 4],
                text: "First document on Rust storage".into(),
                file_name: "rust.rs".into(),
                file_path: "src/rust.rs".into(),
                workspace: "Default".into(),
                last_modified: Some("2026-09-24".into()),
                content_type: Some("Code".into()),
                document_summary: Some("Summary 1".into()),
                keywords: Some("rust, storage".into()),
                content_hash: Some("h1".into()),
                header_path: None,
                start_line: None,
                end_line: None,
            },
            VectorRecord {
                id: "c2".into(),
                vector: vec![0.9; 4],
                text: "Second document on Python storage".into(),
                file_name: "python.py".into(),
                file_path: "src/python.py".into(),
                workspace: "Default".into(),
                last_modified: Some("2026-09-24".into()),
                content_type: Some("Code".into()),
                document_summary: Some("Summary 2".into()),
                keywords: Some("python, storage".into()),
                content_hash: Some("h2".into()),
                header_path: None,
                start_line: None,
                end_line: None,
            },
        ];

        let count = store
            .upsert_records(records, Some("test_chunks"), Some(4))
            .expect("upsert");
        assert_eq!(count, 2);

        let total = store.count_records(None, Some("test_chunks")).expect("count");
        assert_eq!(total, 2);

        // Vector search close to c1
        let results = store
            .search_vector(vec![0.1; 4], 2, Some("Default"), None, Some("test_chunks"))
            .expect("search vector");
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "c1");
        assert!(results[0].score > 0.8);

        // Metadata search
        let meta_results = store
            .search_metadata(
                "file_name = 'python.py'",
                10,
                Some("Default"),
                Some("test_chunks"),
            )
            .expect("search metadata");
        assert_eq!(meta_results.len(), 1);
        assert_eq!(meta_results[0].id, "c2");

        // Delete by file
        store
            .delete_by_file("src/python.py", Some("Default"), Some("test_chunks"))
            .expect("delete by file");
        let after_delete = store
            .count_records(None, Some("test_chunks"))
            .expect("count after delete");
        assert_eq!(after_delete, 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_lance_called_from_within_tokio_runtime() {
        let temp_dir = std::env::temp_dir().join(format!("actx_test_lance_tokio_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        // This is executing directly on a Tokio worker thread.
        // It must NOT panic with "Cannot start a runtime from within a runtime".
        let store = NativeLanceStore::open(&temp_dir).expect("open lance store inside tokio");

        let records = vec![
            VectorRecord {
                id: "tokio_c1".into(),
                vector: vec![0.5; 4],
                text: "Tokio worker thread safety test chunk".into(),
                file_name: "tokio.rs".into(),
                file_path: "src/tokio.rs".into(),
                workspace: "TokioWS".into(),
                last_modified: Some("2026-10-02".into()),
                content_type: Some("Code".into()),
                document_summary: None,
                keywords: None,
                content_hash: Some("th1".into()),
                header_path: None,
                start_line: None,
                end_line: None,
            },
        ];

        let count = store.upsert_records(records, Some("tokio_chunks"), Some(4)).expect("upsert in tokio");
        assert_eq!(count, 1);

        let hits = store.search_metadata("file_name LIKE '%tokio%'", 5, Some("TokioWS"), Some("tokio_chunks"))
            .expect("search_metadata in tokio");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "tokio_c1");
        // Text is encrypted at rest (no plaintext match on disk) and decrypted on read.
        assert_eq!(hits[0].text, "Tokio worker thread safety test chunk");
        let raw_hits = store.search_metadata("text LIKE '%safety%'", 5, Some("TokioWS"), Some("tokio_chunks"))
            .expect("raw text search");
        assert!(raw_hits.is_empty(), "chunk text must not be stored in plaintext");

        let vec_hits = store.search_vector(vec![0.5; 4], 1, Some("TokioWS"), None, Some("tokio_chunks"))
            .expect("search_vector in tokio");
        assert_eq!(vec_hits.len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    const CANARY: &str = "ZEBRA-CANARY-4711";

    fn temp_store(tag: &str) -> (PathBuf, NativeLanceStore) {
        let dir = std::env::temp_dir().join(format!("actx_test_lance_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = NativeLanceStore::open(&dir).expect("open lance store");
        (dir, store)
    }

    fn legacy_record(id: &str, path: &str, workspace: &str, summary: Option<&str>) -> VectorRecord {
        VectorRecord {
            id: id.into(),
            vector: vec![0.25; 4],
            text: format!("Legacy chunk {CANARY} of {path}"),
            file_name: "legacy.md".into(),
            file_path: path.into(),
            workspace: workspace.into(),
            last_modified: Some("2026-10-01".into()),
            content_type: Some("Markdown".into()),
            document_summary: summary.map(String::from),
            keywords: Some(format!("legacy, {CANARY}")),
            content_hash: Some(format!("hash_{id}")),
            header_path: Some(format!("legacy.md > {CANARY}")),
            start_line: Some(3),
            end_line: Some(9),
        }
    }

    fn record_with_vector(id: &str, vector: Vec<f32>) -> VectorRecord {
        VectorRecord {
            id: id.into(),
            vector,
            file_path: format!("docs/{id}.md"),
            ..legacy_record(id, "", "Proj", None)
        }
    }

    /// BUG-021: vectors shorter than the table (Ollama `nomic-embed-text` gives 768 for a 1536
    /// table) are stored zero-padded, and a query of the same short size must still find them,
    /// ranked as in the short space.
    #[test]
    fn test_short_query_vector_finds_padded_rows() {
        let (dir, store) = temp_store("short_query");
        let records = vec![
            record_with_vector("near", vec![1.0, 0.0]),
            record_with_vector("far", vec![0.0, 1.0]),
        ];
        store.upsert_records(records, Some("chunks"), Some(4)).expect("upsert");

        let results = store
            .search_vector(vec![0.9, 0.1], 2, Some("Proj"), None, Some("chunks"))
            .expect("a short query vector is padded, not rejected");
        let _ = std::fs::remove_dir_all(&dir);

        let ids: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, vec!["near", "far"]);
    }

    #[test]
    fn test_longer_query_vector_is_rejected() {
        let (dir, store) = temp_store("long_query");
        store
            .upsert_records(vec![record_with_vector("c", vec![1.0; 4])], Some("chunks"), Some(4))
            .expect("upsert");

        let err = store
            .search_vector(vec![1.0; 6], 1, Some("Proj"), None, Some("chunks"))
            .expect_err("a longer query vector can't be truncated");
        let _ = std::fs::remove_dir_all(&dir);

        assert!(err.contains("6 values") && err.contains("4-value"), "{err}");
    }

    #[test]
    fn test_longer_stored_vector_is_rejected() {
        let (dir, store) = temp_store("long_row");
        let err = store
            .upsert_records(vec![record_with_vector("c", vec![1.0; 6])], Some("chunks"), Some(4))
            .expect_err("a longer vector can't be truncated");
        let exists = store.table_exists("chunks").expect("exists");
        let rows = if exists { store.count_records(None, Some("chunks")).expect("count") } else { 0 };
        let _ = std::fs::remove_dir_all(&dir);

        assert!(err.contains("6-value") && err.contains("4-value"), "{err}");
        assert_eq!(rows, 0);
    }

    fn files_containing(dir: &Path, needle: &[u8]) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            for entry in std::fs::read_dir(&current).expect("read dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if std::fs::read(&path)
                    .map(|bytes| bytes.windows(needle.len()).any(|w| w == needle))
                    .unwrap_or(false)
                {
                    found.push(path);
                }
            }
        }
        found
    }

    #[test]
    fn test_encrypt_plaintext_rows_migrates_legacy_rows() {
        let (dir, store) = temp_store("legacy_migration");
        let table = "legacy_chunks";
        // The same chunk id in two paths and two workspaces, a Windows path written before
        // path normalization, and a row without summary.
        store.write_plaintext_rows_for_test(
            Some(table),
            4,
            vec![
                legacy_record("abc_0", "docs/a.md", "Default", Some("Summary A")),
                legacy_record("abc_0", "docs/b.md", "Default", Some("Summary B")),
                legacy_record("abc_0", "docs/a.md", "Other", None),
                legacy_record("win_0", "C:\\docs\\w.md", "Default", Some("Summary W")),
            ],
        );
        let before = store.search_metadata("text LIKE '%ZEBRA-CANARY%'", 10, None, Some(table)).expect("before");
        assert_eq!(before.len(), 4, "legacy rows are stored in plaintext");
        assert!(!files_containing(&dir, CANARY.as_bytes()).is_empty(), "plaintext is on disk before the migration");

        let report = store.encrypt_plaintext_rows(Some(table)).expect("migration");
        assert_eq!(report, EncryptionMigrationReport { rows_encrypted: 4, versions_pruned: true });

        assert_eq!(store.count_records(None, Some(table)).expect("count"), 4);
        let raw = store
            .search_metadata("text LIKE '%ZEBRA-CANARY%' OR keywords LIKE '%ZEBRA-CANARY%' OR header_path LIKE '%ZEBRA-CANARY%'", 10, None, Some(table))
            .expect("raw");
        assert!(raw.is_empty(), "protected fields must be encrypted");
        let leaked = files_containing(&dir, CANARY.as_bytes());
        assert!(leaked.is_empty(), "plaintext left on disk in {leaked:?}");

        let mut rows = store.search_metadata("id <> ''", 10, None, Some(table)).expect("rows");
        rows.sort_by(|a, b| (&a.workspace, &a.file_path).cmp(&(&b.workspace, &b.file_path)));
        let keys: Vec<_> = rows.iter().map(|r| (r.id.as_str(), r.workspace.as_str(), r.file_path.as_str())).collect();
        assert_eq!(
            keys,
            vec![
                ("win_0", "Default", "C:\\docs\\w.md"),
                ("abc_0", "Default", "docs/a.md"),
                ("abc_0", "Default", "docs/b.md"),
                ("abc_0", "Other", "docs/a.md"),
            ]
        );
        let a = rows.iter().find(|r| r.workspace == "Default" && r.file_path == "docs/a.md").expect("row a");
        assert_eq!(a.text, format!("Legacy chunk {CANARY} of docs/a.md"));
        assert_eq!(a.document_summary.as_deref(), Some("Summary A"));
        assert_eq!(a.keywords.as_deref(), Some(format!("legacy, {CANARY}").as_str()));
        assert_eq!(a.header_path.as_deref(), Some(format!("legacy.md > {CANARY}").as_str()));
        assert_eq!((a.start_line, a.end_line), (Some(3), Some(9)));
        let hits = store.search_vector(vec![0.25; 4], 4, None, None, Some(table)).expect("vector search");
        assert_eq!(hits.len(), 4);
        assert!(hits.iter().all(|h| h.distance < 1e-6), "vectors are unchanged");

        assert_eq!(store.encrypt_plaintext_rows(Some(table)).expect("second run"), EncryptionMigrationReport::default());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_encrypt_plaintext_rows_missing_table_is_noop() {
        let (dir, store) = temp_store("legacy_missing");
        let report = store.encrypt_plaintext_rows(Some("absent_chunks")).expect("noop");
        assert_eq!(report, EncryptionMigrationReport::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_encrypt_plaintext_rows_keeps_encrypted_rows() {
        let (dir, store) = temp_store("legacy_encrypted");
        let table = "encrypted_chunks";
        store
            .upsert_records(vec![legacy_record("enc_0", "docs/e.md", "Default", Some("Summary E"))], Some(table), Some(4))
            .expect("upsert");
        let report = store.encrypt_plaintext_rows(Some(table)).expect("noop");
        assert_eq!(report, EncryptionMigrationReport::default());
        let rows = store.search_metadata("id = 'enc_0'", 1, None, Some(table)).expect("rows");
        assert_eq!(rows[0].text, format!("Legacy chunk {CANARY} of docs/e.md"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_provenance_columns_added_to_legacy_table() {
        let (dir, store) = temp_store("provenance_migration");
        let table = "pre_provenance_chunks";
        store.create_legacy_table_for_test(table, 4, vec![legacy_record("old_0", "docs/old.md", "Default", Some("Old"))]);
        let before = store.search_metadata("id = 'old_0'", 1, None, Some(table)).expect("legacy read");
        assert_eq!((before[0].header_path.as_deref(), before[0].start_line), (None, None));

        // The next write migrates the table in place; old rows keep their vector and get nulls.
        let new = VectorRecord {
            id: "new_0".into(),
            header_path: Some("guide.md > Timeouts".into()),
            start_line: Some(40),
            end_line: Some(58),
            ..legacy_record("new_0", "docs/guide.md", "Default", None)
        };
        store.upsert_records(vec![new], Some(table), Some(4)).expect("upsert after migration");
        assert!(!store.ensure_provenance_columns(Some(table)).expect("idempotent"), "columns already present");

        assert_eq!(store.count_records(None, Some(table)).expect("count"), 2);
        let old = store.search_metadata("id = 'old_0'", 1, None, Some(table)).expect("old row");
        assert_eq!((old[0].header_path.as_deref(), old[0].start_line, old[0].end_line), (None, None, None));
        assert_eq!(old[0].document_summary.as_deref(), Some("Old"));
        let fresh = store.search_metadata("id = 'new_0'", 1, None, Some(table)).expect("new row");
        assert_eq!(fresh[0].header_path.as_deref(), Some("guide.md > Timeouts"));
        assert_eq!((fresh[0].start_line, fresh[0].end_line), (Some(40), Some(58)));
        let hits = store.search_vector(vec![0.25; 4], 2, None, None, Some(table)).expect("vector search");
        assert!(hits.iter().all(|h| h.distance < 1e-6), "vectors are unchanged");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_header_path_encrypted_at_rest() {
        let (dir, store) = temp_store("provenance_encrypted");
        let table = "encrypted_headers";
        store
            .upsert_records(vec![legacy_record("hdr_0", "docs/h.md", "Default", None)], Some(table), Some(4))
            .expect("upsert");
        let raw = store.search_metadata("header_path LIKE '%ZEBRA-CANARY%'", 1, None, Some(table)).expect("raw");
        assert!(raw.is_empty(), "header_path must be stored encrypted");
        let rows = store.search_metadata("id = 'hdr_0'", 1, None, Some(table)).expect("rows");
        assert_eq!(rows[0].header_path.as_deref(), Some(format!("legacy.md > {CANARY}").as_str()));
        assert_eq!((rows[0].start_line, rows[0].end_line), (Some(3), Some(9)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
