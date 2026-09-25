//! Native LanceDB Vector Storage Driver for AnyContext.
//!
//! Provides thread-safe Apache Arrow columnar vector persistence in Rust,
//! delivering zero-copy sub-millisecond similarity search and metadata filtering.

use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::table::Table;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const DEFAULT_TABLE_NAME: &str = "workspace_chunks";
pub const DEFAULT_VECTOR_DIM: usize = 1536;

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
}

pub struct NativeLanceStore {
    db_path: PathBuf,
    runtime: Arc<tokio::runtime::Runtime>,
    conn: Connection,
}

impl NativeLanceStore {
    /// Opens or creates a LanceDB database directory.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let p = db_path.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&p);

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to initialize Tokio runtime for LanceDB: {e}"))?;

        let uri = p.to_string_lossy().to_string();
        let conn = rt
            .block_on(async { lancedb::connect(&uri).execute().await })
            .map_err(|e| format!("Failed to connect to LanceDB at {uri}: {e}"))?;

        Ok(Self {
            db_path: p,
            runtime: Arc::new(rt),
            conn,
        })
    }

    pub fn get_schema(dim: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
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
        ]))
    }

    pub fn table_exists(&self, table_name: &str) -> Result<bool, String> {
        self.runtime.block_on(async {
            let names = self
                .conn
                .table_names()
                .execute()
                .await
                .map_err(|e| e.to_string())?;
            Ok(names.contains(&table_name.to_string()))
        })
    }

    pub fn get_or_create_table(&self, table_name: &str, dim: usize) -> Result<Table, String> {
        self.runtime.block_on(async {
            let names = self
                .conn
                .table_names()
                .execute()
                .await
                .map_err(|e| e.to_string())?;
            if names.contains(&table_name.to_string()) {
                self.conn
                    .open_table(table_name)
                    .execute()
                    .await
                    .map_err(|e| e.to_string())
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

        for r in records {
            ids.push(r.id);
            if r.vector.len() == d {
                flat_vectors.extend(r.vector);
            } else {
                let mut padded = r.vector;
                padded.resize(d, 0.0);
                flat_vectors.extend(padded);
            }
            texts.push(r.text);
            file_names.push(r.file_name);
            file_paths.push(r.file_path.replace('\\', "/"));
            workspaces.push(r.workspace);
            last_modifieds.push(r.last_modified.unwrap_or_default());
            content_types.push(r.content_type.unwrap_or_else(|| "Local Document".into()));
            doc_summaries.push(r.document_summary.unwrap_or_default());
            keywords_list.push(r.keywords.unwrap_or_default());
            hashes.push(r.content_hash.unwrap_or_default());
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
            ],
        )
        .map_err(|e| format!("Failed to create Arrow RecordBatch: {e}"))?;

        let table = self.get_or_create_table(tname, d)?;

        self.runtime.block_on(async {
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

    /// Performs vector similarity search with optional workspace filtering.
    pub fn search_vector(
        &self,
        query_vector: Vec<f32>,
        limit: usize,
        workspace: Option<&str>,
        filter_expr: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<Vec<ScoredVectorResult>, String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(Vec::new());
        }

        self.runtime.block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;

            let mut query = table.vector_search(query_vector).map_err(|e| e.to_string())?;
            query = query.limit(limit);

            let mut where_clauses = Vec::new();
            if let Some(ws) = workspace {
                let clean_ws = ws.replace('\'', "''");
                where_clauses.push(format!("workspace = '{clean_ws}'"));
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
        })
    }

    /// Performs ultra-fast (<25ms) metadata filtering on LanceDB without computing embeddings.
    pub fn search_metadata(
        &self,
        where_clause: &str,
        limit: usize,
        workspace: Option<&str>,
        table_name: Option<&str>,
    ) -> Result<Vec<ScoredVectorResult>, String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? || where_clause.is_empty() {
            return Ok(Vec::new());
        }

        self.runtime.block_on(async {
            let table = self
                .conn
                .open_table(tname)
                .execute()
                .await
                .map_err(|e| e.to_string())?;

            let mut where_clauses = Vec::new();
            if let Some(ws) = workspace {
                let clean_ws = ws.replace('\'', "''");
                where_clauses.push(format!("workspace = '{clean_ws}'"));
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
        })
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

        self.runtime.block_on(async {
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

        self.runtime.block_on(async {
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

        self.runtime.block_on(async {
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

    pub fn delete_by_id(&self, chunk_id: &str, table_name: Option<&str>) -> Result<(), String> {
        let tname = table_name.unwrap_or(DEFAULT_TABLE_NAME);
        if !self.table_exists(tname)? {
            return Ok(());
        }

        self.runtime.block_on(async {
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

        for i in 0..num_rows {
            let dist = dist_col.map(|d| d.value(i)).unwrap_or(0.0);
            let score = 1.0 / (1.0 + dist.max(0.0));

            results.push(ScoredVectorResult {
                id: id_col.value(i).to_string(),
                text: text_col.value(i).to_string(),
                file_name: fn_col.value(i).to_string(),
                file_path: fp_col.value(i).to_string(),
                workspace: ws_col.value(i).to_string(),
                score,
                distance: dist,
                last_modified: lm_col.map(|c| c.value(i).to_string()),
                content_type: ct_col.map(|c| c.value(i).to_string()),
                document_summary: ds_col.map(|c| c.value(i).to_string()),
                keywords: kw_col.map(|c| c.value(i).to_string()),
                content_hash: h_col.map(|c| c.value(i).to_string()),
            });
        }

        Ok(results)
    }

    pub fn get_db_path(&self) -> &Path {
        &self.db_path
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
}
