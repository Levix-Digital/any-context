use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

use super::bm25::BM25Index;
use super::rrf::reciprocal_rank_fusion;
use super::diversifier::{RankedChunk, apply_source_diversification, apply_density_budget};

#[pyclass]
pub struct HybridRetrieverEngine {
    bm25: BM25Index,
}

#[pymethods]
impl HybridRetrieverEngine {
    #[new]
    #[pyo3(signature = (k1=None, b=None))]
    pub fn new(k1: Option<f32>, b: Option<f32>) -> Self {
        Self {
            bm25: BM25Index::new(k1, b),
        }
    }

    /// Adds a document chunk into the in-memory BM25 index.
    #[pyo3(signature = (id, text, file_name, file_path, workspace="Default".to_string(), content_type="Local Document".to_string()))]
    pub fn add_chunk(
        &mut self,
        id: String,
        text: String,
        file_name: String,
        file_path: String,
        workspace: String,
        content_type: String,
    ) {
        self.bm25.add_chunk(id, text, file_name, file_path, workspace, content_type);
    }

    /// Batch adds multiple chunk records into the in-memory BM25 index.
    pub fn add_chunks_batch(&mut self, chunks: Vec<Bound<'_, PyDict>>) -> PyResult<usize> {
        let mut count = 0;
        for c in chunks {
            let id: String = c.get_item("id")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            let text: String = c.get_item("text")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            let file_name: String = c.get_item("file_name")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            let file_path: String = c.get_item("file_path")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            let workspace: String = c.get_item("workspace")?.map(|v| v.extract()).transpose()?.unwrap_or_else(|| "Default".to_string());
            let content_type: String = c.get_item("content_type")?.map(|v| v.extract()).transpose()?.unwrap_or_else(|| "Local Document".to_string());

            if !id.is_empty() && !text.is_empty() {
                self.bm25.add_chunk(id, text, file_name, file_path, workspace, content_type);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Removes chunk by ID.
    pub fn remove_by_id(&mut self, id: &str) -> bool {
        self.bm25.remove_by_id(id)
    }

    /// Removes all chunks for a file path or URL.
    pub fn remove_by_file(&mut self, file_path: &str) -> usize {
        self.bm25.remove_by_file(file_path)
    }

    /// Removes all chunks for a workspace.
    pub fn remove_by_workspace(&mut self, workspace: &str) -> usize {
        self.bm25.remove_by_workspace(workspace)
    }

    /// Executes BM25 search alone.
    #[pyo3(signature = (query, limit=50, workspace=None))]
    pub fn search_bm25(&self, query: &str, limit: usize, workspace: Option<&str>) -> Vec<(String, f32)> {
        self.bm25.search(query, limit, workspace)
    }

    /// Returns total chunks in BM25 index.
    pub fn count_bm25_chunks(&self) -> usize {
        self.bm25.len()
    }

    /// Saves BM25 index to binary file on disk.
    pub fn save_bm25_to_file(&self, path: &str) -> PyResult<()> {
        self.bm25
            .save_to_file(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e))
    }

    /// Loads BM25 index from binary file on disk.
    pub fn load_bm25_from_file(&mut self, path: &str) -> PyResult<()> {
        let loaded = BM25Index::load_from_file(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e))?;
        self.bm25 = loaded;
        Ok(())
    }

    /// Executes full Hybrid Retrieval with Reciprocal Rank Fusion (RRF), Source-Fair Round-Robin,
    /// and Density Budgeting 100% in native Rust.
    #[pyo3(signature = (
        dense_results,
        query,
        workspace=None,
        candidate_pool_k=100,
        target_top_k=20,
        max_per_source=3,
        max_density_chars=40000,
        rrf_k=Some(60)
    ))]
    pub fn fuse_and_diversify(
        &self,
        py: Python<'_>,
        dense_results: Vec<Bound<'_, PyDict>>,
        query: &str,
        workspace: Option<String>,
        candidate_pool_k: usize,
        target_top_k: usize,
        max_per_source: usize,
        max_density_chars: usize,
        rrf_k: Option<usize>,
    ) -> PyResult<Vec<PyObject>> {
        // 1. Build dense rankings list & map of dense metadata
        let mut dense_ids = Vec::with_capacity(dense_results.len());
        let mut dense_map: HashMap<String, RankedChunk> = HashMap::with_capacity(dense_results.len());

        for (_idx, d) in dense_results.iter().enumerate() {
            let id: String = d.get_item("id")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            dense_ids.push(id.clone());

            let text: String = d.get_item("text")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            let file_name: String = d.get_item("file_name")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            let file_path: String = d.get_item("file_path")?.map(|v| v.extract()).transpose()?.unwrap_or_default();
            let ws: String = d.get_item("workspace")?.map(|v| v.extract()).transpose()?.unwrap_or_else(|| "Default".to_string());
            let content_type: String = d.get_item("content_type")?.map(|v| v.extract()).transpose()?.unwrap_or_else(|| "Local Document".to_string());
            let score: f64 = d.get_item("score")?.map(|v| v.extract()).transpose()?.unwrap_or(0.0);

            dense_map.insert(
                id.clone(),
                RankedChunk {
                    id,
                    score,
                    file_path,
                    file_name,
                    text,
                    workspace: ws,
                    content_type,
                },
            );
        }

        // 2. Query BM25 index in Rust
        let sparse_matches = self.bm25.search(query, candidate_pool_k, workspace.as_deref());
        let sparse_ids: Vec<String> = sparse_matches.into_iter().map(|(id, _)| id).collect();

        // 3. Reciprocal Rank Fusion (RRF)
        let fused_results = reciprocal_rank_fusion(&dense_ids, &sparse_ids, rrf_k);

        // 4. Assemble candidate chunks with RRF scores
        let mut candidates: Vec<RankedChunk> = Vec::with_capacity(fused_results.len());
        for res in fused_results {
            if let Some(mut chunk) = dense_map.remove(&res.id) {
                chunk.score = res.score;
                candidates.push(chunk);
            } else if let Some(doc) = self.bm25.get_doc_by_id(&res.id) {
                candidates.push(RankedChunk {
                    id: doc.id.clone(),
                    score: res.score,
                    file_path: doc.file_path.clone(),
                    file_name: doc.file_name.clone(),
                    text: doc.text.clone(),
                    workspace: doc.workspace.clone(),
                    content_type: doc.content_type.clone(),
                });
            }
        }

        // 5. Source-Fair Round-Robin Diversification
        let diversified = apply_source_diversification(candidates, max_per_source, target_top_k);

        // 6. Density Budgeting
        let budgeted = apply_density_budget(diversified, max_density_chars);

        // 7. Convert into Python dictionaries
        let mut py_results = Vec::with_capacity(budgeted.len());
        for chunk in budgeted {
            let dict = PyDict::new_bound(py);
            dict.set_item("id", chunk.id)?;
            dict.set_item("score", chunk.score)?;
            dict.set_item("text", chunk.text)?;
            dict.set_item("file_name", chunk.file_name)?;
            dict.set_item("file_path", chunk.file_path)?;
            dict.set_item("workspace", chunk.workspace)?;
            dict.set_item("content_type", chunk.content_type)?;
            py_results.push(dict.into());
        }

        Ok(py_results)
    }
}
