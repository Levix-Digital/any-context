use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use serde::{Deserialize, Serialize};

use super::tokenizer::tokenize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Posting {
    pub doc_idx: u32,
    pub term_frequency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocRecord {
    pub id: String,
    pub text: String,
    pub file_name: String,
    pub file_path: String,
    pub workspace: String,
    pub content_type: String,
    pub token_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BM25Index {
    pub docs: Vec<DocRecord>,
    pub doc_id_to_idx: HashMap<String, u32>,
    pub postings: HashMap<String, Vec<Posting>>,
    pub avg_doc_len: f32,
    pub k1: f32,
    pub b: f32,
}

impl BM25Index {
    pub fn new(k1: Option<f32>, b: Option<f32>) -> Self {
        Self {
            docs: Vec::new(),
            doc_id_to_idx: HashMap::new(),
            postings: HashMap::new(),
            avg_doc_len: 0.0,
            k1: k1.unwrap_or(1.2),
            b: b.unwrap_or(0.75),
        }
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Adds or updates a document chunk into the index.
    pub fn add_chunk(
        &mut self,
        id: String,
        text: String,
        file_name: String,
        file_path: String,
        workspace: String,
        content_type: String,
    ) {
        // If document already exists, remove it first to allow clean update
        if self.doc_id_to_idx.contains_key(&id) {
            self.remove_by_id(&id);
        }

        let tokens = tokenize(&text);
        let token_count = tokens.len() as u32;

        let doc_idx = self.docs.len() as u32;
        self.docs.push(DocRecord {
            id: id.clone(),
            text,
            file_name,
            file_path,
            workspace,
            content_type,
            token_count,
        });
        self.doc_id_to_idx.insert(id, doc_idx);

        // Count term frequencies within this document
        let mut tf_map: HashMap<String, u32> = HashMap::new();
        for t in tokens {
            *tf_map.entry(t).or_insert(0) += 1;
        }

        for (term, tf) in tf_map {
            self.postings.entry(term).or_default().push(Posting {
                doc_idx,
                term_frequency: tf,
            });
        }

        self.recalculate_avg_len();
    }

    /// Recalculates average document length across active records.
    fn recalculate_avg_len(&mut self) {
        if self.docs.is_empty() {
            self.avg_doc_len = 0.0;
        } else {
            let total_tokens: u64 = self.docs.iter().map(|d| d.token_count as u64).sum();
            self.avg_doc_len = (total_tokens as f32) / (self.docs.len() as f32);
        }
    }

    /// Removes a document chunk by ID and cleanly updates the index.
    pub fn remove_by_id(&mut self, id: &str) -> bool {
        if !self.doc_id_to_idx.contains_key(id) {
            return false;
        }
        self.docs.retain(|d| d.id != id);
        self.rebuild_postings();
        true
    }

    /// Purges all document chunks associated with a specific workspace.
    pub fn remove_by_workspace(&mut self, workspace: &str) -> usize {
        let before = self.docs.len();
        self.docs.retain(|d| d.workspace != workspace);
        let removed = before - self.docs.len();
        if removed > 0 {
            self.rebuild_postings();
        }
        removed
    }

    /// Purges all document chunks associated with a specific file path or URL.
    pub fn remove_by_file(&mut self, file_path: &str) -> usize {
        let norm_target = file_path.replace('\\', "/");
        let before = self.docs.len();
        self.docs.retain(|d| {
            let d_norm = d.file_path.replace('\\', "/");
            d_norm != norm_target && !d_norm.starts_with(&format!("{}/", norm_target.trim_end_matches('/')))
        });
        let removed = before - self.docs.len();
        if removed > 0 {
            self.rebuild_postings();
        }
        removed
    }

    /// Clears and rebuilds inverted postings map following deletions.
    fn rebuild_postings(&mut self) {
        self.doc_id_to_idx.clear();
        self.postings.clear();

        for (i, d) in self.docs.iter().enumerate() {
            let doc_idx = i as u32;
            self.doc_id_to_idx.insert(d.id.clone(), doc_idx);

            let tokens = tokenize(&d.text);
            let mut tf_map: HashMap<String, u32> = HashMap::new();
            for t in tokens {
                *tf_map.entry(t).or_insert(0) += 1;
            }
            for (term, tf) in tf_map {
                self.postings.entry(term).or_default().push(Posting {
                    doc_idx,
                    term_frequency: tf,
                });
            }
        }
        self.recalculate_avg_len();
    }

    /// Retrieves document record by ID.
    pub fn get_doc_by_id(&self, id: &str) -> Option<&DocRecord> {
        self.doc_id_to_idx.get(id).and_then(|&idx| self.docs.get(idx as usize))
    }

    /// Executes Okapi BM25 ranking against the inverted index.
    /// Returns a list of (doc_id, score) pairs sorted descending by BM25 score.
    pub fn search(&self, query: &str, limit: usize, workspace: Option<&str>) -> Vec<(String, f32)> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() || self.docs.is_empty() {
            return Vec::new();
        }

        let n = self.docs.len() as f32;
        let avg_dl = if self.avg_doc_len > 0.0 { self.avg_doc_len } else { 1.0 };
        let mut scores: HashMap<u32, f32> = HashMap::new();

        for term in &query_tokens {
            if let Some(plist) = self.postings.get(term) {
                let doc_freq = plist.len() as f32;
                // Robertson-Spärck Jones IDF with +1 smoothing: ln(1 + (N - df + 0.5) / (df + 0.5))
                let idf = ((n - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();

                for p in plist {
                    let doc_idx = p.doc_idx;
                    if let Some(doc) = self.docs.get(doc_idx as usize) {
                        if let Some(ws) = workspace {
                            if !ws.is_empty() && doc.workspace != ws && ws != "Default" {
                                continue;
                            }
                        }

                        let tf = p.term_frequency as f32;
                        let dl = doc.token_count as f32;
                        let denom = tf + self.k1 * (1.0 - self.b + self.b * (dl / avg_dl));
                        let term_score = idf * (tf * (self.k1 + 1.0)) / denom;

                        *scores.entry(doc_idx).or_insert(0.0) += term_score;
                    }
                }
            }
        }

        let mut results: Vec<(String, f32)> = scores
            .into_iter()
            .filter_map(|(doc_idx, score)| {
                self.docs.get(doc_idx as usize).map(|d| (d.id.clone(), score))
            })
            .collect();

        // Sort descending by score
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        if results.len() > limit {
            results.truncate(limit);
        }

        results
    }

    /// Persists the index to disk using high-speed bincode serialization.
    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let file = File::create(path).map_err(|e| format!("Failed to create BM25 index file '{}': {}", path, e))?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, self)
            .map_err(|e| format!("Failed to serialize BM25 index to '{}': {}", path, e))?;
        Ok(())
    }

    /// Loads the index from disk using bincode deserialization.
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open BM25 index file '{}': {}", path, e))?;
        let reader = BufReader::new(file);
        let index: Self = bincode::deserialize_from(reader)
            .map_err(|e| format!("Failed to deserialize BM25 index from '{}': {}", path, e))?;
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_exact_match_ranking() {
        let mut index = BM25Index::new(None, None);

        index.add_chunk(
            "chunk_1".to_string(),
            "DATABASE_URL=postgres://user:pass@localhost:5432/mydb".to_string(),
            ".env".to_string(),
            ".env".to_string(),
            "Default".to_string(),
            "Configuration / Env".to_string(),
        );

        index.add_chunk(
            "chunk_2".to_string(),
            "The system connects to database using credentials.".to_string(),
            "readme.md".to_string(),
            "readme.md".to_string(),
            "Default".to_string(),
            "Markdown Document".to_string(),
        );

        // Search for exact technical term
        let results = index.search("DATABASE_URL", 5, None);
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "chunk_1");
        assert!(results[0].1 > 0.5);
    }

    #[test]
    fn test_bm25_workspace_isolation() {
        let mut index = BM25Index::new(None, None);

        index.add_chunk(
            "ws1_c1".to_string(),
            "Deploy instructions for server alpha".to_string(),
            "deploy.sh".to_string(),
            "deploy.sh".to_string(),
            "WorkspaceA".to_string(),
            "Shell Script".to_string(),
        );

        index.add_chunk(
            "ws2_c1".to_string(),
            "Deploy instructions for server beta".to_string(),
            "deploy.sh".to_string(),
            "deploy.sh".to_string(),
            "WorkspaceB".to_string(),
            "Shell Script".to_string(),
        );

        let res_a = index.search("server", 5, Some("WorkspaceA"));
        assert_eq!(res_a.len(), 1);
        assert_eq!(res_a[0].0, "ws1_c1");

        let res_b = index.search("server", 5, Some("WorkspaceB"));
        assert_eq!(res_b.len(), 1);
        assert_eq!(res_b[0].0, "ws2_c1");
    }

    #[test]
    fn test_bm25_serialization_roundtrip() {
        let mut index = BM25Index::new(None, None);
        index.add_chunk(
            "c1".to_string(),
            "Docker container build instruction FROM alpine:latest".to_string(),
            "Dockerfile".to_string(),
            "Dockerfile".to_string(),
            "Default".to_string(),
            "Container / Build Definition".to_string(),
        );

        let tmp_path = std::env::temp_dir().join("bm25_test_index.bin");
        let path_str = tmp_path.to_str().unwrap();

        index.save_to_file(path_str).expect("Failed to save BM25 index");
        let loaded = BM25Index::load_from_file(path_str).expect("Failed to load BM25 index");

        assert_eq!(loaded.len(), 1);
        let res = loaded.search("alpine", 5, None);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0, "c1");

        let _ = std::fs::remove_file(tmp_path);
    }
}
