use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPayload {
    pub id: String,
    pub text: String,
    pub file_name: String,
    pub file_path: String,
    pub header_path: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
    pub content_type: String,
    pub chunk_index: usize,
}

impl ChunkPayload {
    pub fn new(
        id: String,
        text: String,
        file_name: String,
        file_path: String,
        header_path: Option<String>,
        start_line: usize,
        end_line: usize,
        content_type: String,
        chunk_index: usize,
    ) -> Self {
        Self {
            id,
            text,
            file_name,
            file_path,
            header_path,
            start_line,
            end_line,
            content_type,
            chunk_index,
        }
    }
}

/// Rich semantic metadata envelope extracted per document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEnvelope {
    pub summary: String,
    pub keywords: Vec<String>,
    pub content_hash: String,
    pub file_name: String,
    pub file_path: Option<String>,
    pub url: Option<String>,
    pub created_at: String,
}

