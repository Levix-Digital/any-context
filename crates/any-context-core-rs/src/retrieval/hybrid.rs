use super::bm25::BM25Index;

pub struct HybridRetrieverEngine {
    bm25: BM25Index,
}

impl HybridRetrieverEngine {
    pub fn new(k1: Option<f32>, b: Option<f32>) -> Self {
        Self {
            bm25: BM25Index::new(k1, b),
        }
    }

    pub fn add_chunk(
        &mut self,
        id: String,
        text: String,
        file_name: String,
        file_path: String,
        workspace: String,
        content_type: String,
        last_modified: Option<String>,
    ) {
        self.bm25.add_chunk(id, text, file_name, file_path, workspace, content_type, last_modified);
    }

    pub fn remove_by_id(&mut self, id: &str) -> bool {
        self.bm25.remove_by_id(id)
    }

    pub fn remove_by_file(&mut self, file_path: &str) -> usize {
        self.bm25.remove_by_file(file_path)
    }

    pub fn remove_by_workspace(&mut self, workspace: &str) -> usize {
        self.bm25.remove_by_workspace(workspace)
    }

    pub fn search_bm25(&self, query: &str, limit: usize, workspace: Option<&str>) -> Vec<(String, f32)> {
        self.bm25.search(query, limit, workspace)
    }

    pub fn count_bm25_chunks(&self) -> usize {
        self.bm25.len()
    }

    pub fn save_bm25_to_file(&self, path: &str) -> std::io::Result<()> {
        self.bm25.save_to_file(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    pub fn load_bm25_from_file(&mut self, path: &str) -> std::io::Result<()> {
        match BM25Index::load_from_file(path) {
            Ok(loaded) => {
                self.bm25 = loaded;
                Ok(())
            }
            Err(e) => {
                let corrupt_name = format!("{}.corrupt.{}", path, chrono::Utc::now().timestamp());
                let _ = std::fs::rename(path, &corrupt_name);
                self.bm25 = BM25Index::new(None, None);
                Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!(
                    "BM25 index at '{}' was corrupted and quarantined to '{}': {}",
                    path, corrupt_name, e
                )))
            }
        }
    }
}
