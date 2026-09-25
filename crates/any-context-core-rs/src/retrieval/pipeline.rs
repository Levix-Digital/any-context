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
use actx_lm::traits::LmProvider;

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
}

/// Unified Native Hybrid Retrieval and Reranking Pipeline.
pub struct NativeHybridPipeline {
    db_path: PathBuf,
    lance_store: Arc<NativeLanceStore>,
    bm25_indices: Arc<RwLock<HashMap<String, BM25Index>>>,
    lm_client: Option<Arc<dyn LmProvider>>,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl NativeHybridPipeline {
    /// Opens or connects to the underlying storage at `db_path`.
    pub fn open(db_path: impl AsRef<Path>) -> Result<Self, String> {
        let p = db_path.as_ref().to_path_buf();
        let lance_store = Arc::new(NativeLanceStore::open(&p)?);
        let runtime = lance_store.runtime().clone();
        Ok(Self {
            db_path: p,
            lance_store,
            bm25_indices: Arc::new(RwLock::new(HashMap::new())),
            lm_client: None,
            runtime,
        })
    }

    /// Creates pipeline with an existing [`NativeLanceStore`] and optional [`LmProvider`].
    pub fn new(lance_store: Arc<NativeLanceStore>, lm_client: Option<Arc<dyn LmProvider>>) -> Self {
        let p = lance_store.db_path().to_path_buf();
        let runtime = lance_store.runtime().clone();
        Self {
            db_path: p,
            lance_store,
            bm25_indices: Arc::new(RwLock::new(HashMap::new())),
            lm_client,
            runtime,
        }
    }

    /// Sets the optional language model client for native on-the-fly embeddings.
    pub fn set_lm_client(&mut self, lm_client: Option<Arc<dyn LmProvider>>) {
        self.lm_client = lm_client;
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
            BM25Index::load_from_file(file_path.to_str().unwrap_or(""))
                .unwrap_or_else(|_| BM25Index::new(None, None))
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
    ) {
        // Ensure index is loaded
        let _ = self.get_or_load_bm25(table_name);
        let mut writer = self.bm25_indices.write().unwrap();
        if let Some(idx) = writer.get_mut(table_name) {
            idx.add_chunk(id, text, file_name, file_path, workspace, content_type);
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

        // 5. Query Vector Resolution (Passed in or computed via actx-lm)
        let query_vec = if let Some(ref v) = req.query_vector {
            Some(v.clone())
        } else if let Some(ref lm) = self.lm_client {
            match lm.embed("text-embedding-3-small", &[req.query_text.clone()]).await {
                Ok(vecs) if !vecs.is_empty() => Some(vecs[0].clone()),
                _ => None,
            }
        } else {
            None
        };

        // 6. Dense Vector Similarity Search
        if let Some(q_vec) = query_vec {
            for (ws_name, limit) in &targets {
                let ws_filter = if ws_name == "Default" {
                    None
                } else {
                    Some(ws_name.as_str())
                };
                if let Ok(matches) = self
                    .lance_store
                    .search_vector_async(q_vec.clone(), *limit, ws_filter, None, Some(&req.table_name))
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

            // Linked shared sources search
            if !req.linked_sources.is_empty() {
                let clauses: Vec<String> = req
                    .linked_sources
                    .iter()
                    .map(|l| format!("file_path LIKE '%{l}%'"))
                    .collect();
                let filter_expr = clauses.join(" OR ");
                if let Ok(matches) = self
                    .lance_store
                    .search_vector_async(
                        q_vec.clone(),
                        req.candidate_pool_k * 2,
                        None,
                        Some(&filter_expr),
                        Some(&req.table_name),
                    )
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

            let (file_name, file_path, workspace, text, content_type, content_hash, dense_score) =
                if let Some(ref d) = dense_entry {
                    (
                        d.file_name.clone(),
                        d.file_path.clone(),
                        d.workspace.clone(),
                        d.text.clone(),
                        d.content_type.clone().unwrap_or_else(|| "Local Document".to_string()),
                        d.content_hash.clone().unwrap_or_default(),
                        Some(d.score),
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
                    )
                } else {
                    continue;
                };

            let token_count = estimate_token_count(&text);
            let is_guaranteed = guaranteed_file_cids.contains(&chunk_id);
            let final_score = if is_guaranteed { 1.0 } else { res.score };

            results.push(HybridSearchResult {
                chunk_id,
                file_name,
                file_path,
                workspace,
                text,
                content_type,
                content_hash,
                score: final_score,
                dense_score,
                sparse_score,
                token_count,
                matched_subqueries: vec![sub_query_tag.clone()],
            });
        }

        Ok(results)
    }

    /// Single-query hybrid retrieval synchronous wrapper.
    pub fn search(&self, req: HybridSearchRequest) -> Result<Vec<HybridSearchResult>, String> {
        self.runtime.block_on(self.search_single_async(&req))
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
                    // Accumulate RRF score across sub-queries (RFC-042 multi-query boost)
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

        // 3. Filter by min_score if set
        if first_req.min_score > 0.0 {
            candidates.retain(|c| c.score >= first_req.min_score);
        }

        // Sort descending by accumulated RRF score
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
        self.runtime
            .block_on(self.retrieve_hybrid_batch_async(&requests))
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
        );

        pipeline.add_chunk_to_bm25(
            "workspace_chunks",
            "chunk-b".to_string(),
            "database connection pool with SQL query execution".to_string(),
            "db.rs".to_string(),
            "/src/db.rs".to_string(),
            "Default".to_string(),
            "Local Document".to_string(),
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
}
