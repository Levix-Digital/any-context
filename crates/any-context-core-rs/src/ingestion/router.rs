use std::path::Path;
use pyo3::prelude::*;
use crate::ingestion::chunkers::code::ASTCodeChunker;
use crate::ingestion::chunkers::markdown::MarkdownHeaderChunker;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

#[pyclass]
#[derive(Debug, Clone)]
pub struct IngestionRouter {
    pub markdown_chunker: MarkdownHeaderChunker,
    pub code_chunker: ASTCodeChunker,
}

#[pymethods]
impl IngestionRouter {
    #[new]
    #[pyo3(signature = (max_chunk_chars=1800, overlap_chars=200))]
    pub fn new(max_chunk_chars: usize, overlap_chars: usize) -> Self {
        Self {
            markdown_chunker: MarkdownHeaderChunker::new(max_chunk_chars, overlap_chars),
            code_chunker: ASTCodeChunker::new(max_chunk_chars),
        }
    }

    /// Determines whether the given file path or extension has a specialized Rust chunker available.
    pub fn supports_file(&self, file_path: &str) -> bool {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        matches!(ext.as_str(), "md" | "markdown" | "rst" | "mdown") || self.code_chunker.supports_extension(&ext)
    }

    /// Chunks document content using the specialized parser matching the file extension.
    pub fn chunk_text(&self, file_path: &str, content: &str) -> PyResult<Vec<ChunkPayload>> {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "md" | "markdown" | "rst" | "mdown" => {
                self.markdown_chunker.chunk(file_path, content).map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Markdown chunker error: {}", e))
                })
            }
            "py" | "pyw" | "pyi" => {
                self.code_chunker.chunk(file_path, content).map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Code AST chunker error: {}", e))
                })
            }
            _ => Ok(Vec::new()),
        }
    }

    /// Reads and chunks a file directly from the filesystem in high-speed native Rust.
    pub fn chunk_file(&self, file_path: &str) -> PyResult<Vec<ChunkPayload>> {
        let content = std::fs::read_to_string(file_path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to read file '{}': {}", file_path, e))
        })?;
        self.chunk_text(file_path, &content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_supports_files() {
        let router = IngestionRouter::new(1800, 200);
        assert!(router.supports_file("README.md"));
        assert!(router.supports_file("docs/architecture.markdown"));
        assert!(router.supports_file("/path/to/spec.rst"));
        assert!(router.supports_file("main.py"));
        assert!(router.supports_file("src/utils.pyw"));
        assert!(router.supports_file("stubs.pyi"));
        assert!(!router.supports_file("report.pdf"));
        assert!(!router.supports_file("data.xlsx"));
    }

    #[test]
    fn test_router_chunk_text() {
        let router = IngestionRouter::new(1800, 200);
        let md = "# Title\nParagraph content.";
        let chunks = router.chunk_text("test.md", md).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].header_path.as_deref(), Some("# Title"));
    }

    #[test]
    fn test_router_chunk_python_code() {
        let router = IngestionRouter::new(1800, 200);
        let code = "def hello():\n    print('world')\n";
        let chunks = router.chunk_text("script.py", code).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].header_path.as_deref(), Some("def hello"));
        assert!(chunks[0].text.contains("// Context: script.py > def hello"));
    }
}
