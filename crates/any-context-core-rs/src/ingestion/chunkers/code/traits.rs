use crate::models::ChunkPayload;

/// Trait defining an AST-aware code chunker for a specific programming language.
pub trait LanguageASTParser: Send + Sync {
    /// Parse the given code content and emit semantic code chunks with rich symbol breadcrumbs.
    fn parse_chunks(
        &self,
        file_path: &str,
        content: &str,
        max_chunk_chars: usize,
    ) -> Result<Vec<ChunkPayload>, String>;
}
