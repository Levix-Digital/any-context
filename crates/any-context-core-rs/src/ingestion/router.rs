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
        if matches!(ext.as_str(), "md" | "markdown" | "rst" | "mdown") {
            self.markdown_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Markdown chunker error: {}", e))
            })
        } else if self.code_chunker.supports_extension(&ext) {
            self.code_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Code AST chunker error: {}", e))
            })
        } else {
            Ok(Vec::new())
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
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        assert!(router.supports_file("README.md"));
        assert!(router.supports_file("docs/architecture.markdown"));
        assert!(router.supports_file("/path/to/spec.rst"));
        assert!(router.supports_file("main.py"));
        assert!(router.supports_file("src/utils.pyw"));
        assert!(router.supports_file("stubs.pyi"));
        assert!(router.supports_file("src/index.ts"));
        assert!(router.supports_file("src/App.tsx"));
        assert!(router.supports_file("server.js"));
        assert!(router.supports_file("component.jsx"));
        assert!(router.supports_file("UserService.java"));
        assert!(router.supports_file("OrderController.cs"));
        assert!(router.supports_file("main.go"));
        assert!(router.supports_file("src/lib.rs"));
        assert!(router.supports_file("kernel.c"));
        assert!(router.supports_file("header.h"));
        assert!(router.supports_file("engine.cpp"));
        assert!(router.supports_file("include/engine.hpp"));
        assert!(!router.supports_file("report.pdf"));
        assert!(!router.supports_file("data.xlsx"));
    }

    #[test]
    fn test_router_chunk_text() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let md = "# Title\nParagraph content.";
        let chunks = router.chunk_text("test.md", md).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].header_path.as_deref(), Some("# Title"));
    }

    #[test]
    fn test_router_chunk_python_code() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "def hello():\n    print('world')\n";
        let chunks = router.chunk_text("script.py", code).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].header_path.as_deref(), Some("def hello"));
        assert!(chunks[0].text.contains("// Context: script.py > def hello"));
    }

    #[test]
    fn test_router_chunk_typescript() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "export function calculateTotal(items: number[]): number {\n  return items.reduce((a, b) => a + b, 0);\n}\n";
        let chunks = router.chunk_text("math.ts", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: math.ts > function calculateTotal")));
    }

    #[test]
    fn test_router_chunk_java() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "public class Calculator {\n    public int add(int a, int b) {\n        return a + b;\n    }\n}\n";
        let chunks = router.chunk_text("Calculator.java", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: Calculator.java > class Calculator")));
    }

    #[test]
    fn test_router_chunk_csharp() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "namespace MyStore;\npublic class OrderProcessor {\n    public void Process() {}\n}\n";
        let chunks = router.chunk_text("OrderProcessor.cs", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: OrderProcessor.cs > class OrderProcessor")));
    }

    #[test]
    fn test_router_chunk_go() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "package main\n\nfunc RunApp() error {\n    return nil\n}\n";
        let chunks = router.chunk_text("main.go", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: main.go > function RunApp")));
    }

    #[test]
    fn test_router_chunk_rust() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "pub fn execute_task() -> bool {\n    true\n}\n";
        let chunks = router.chunk_text("task.rs", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: task.rs > fn execute_task")));
    }

    #[test]
    fn test_router_chunk_cpp() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "#include <iostream>\n\nclass Engine {\npublic:\n    void start();\n};\n";
        let chunks = router.chunk_text("Engine.cpp", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: Engine.cpp > class Engine")));
    }
}
