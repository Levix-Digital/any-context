use crate::models::ChunkPayload;

pub trait Chunker: Send + Sync {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String>;
}
