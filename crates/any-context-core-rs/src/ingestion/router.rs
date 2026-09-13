use std::path::Path;
use pyo3::prelude::*;
use crate::ingestion::chunkers::code::ASTCodeChunker;
use crate::ingestion::chunkers::markdown::MarkdownHeaderChunker;
use crate::ingestion::chunkers::structured::StructuredDataChunker;
use crate::ingestion::chunkers::tabular::TabularChunker;
use crate::ingestion::chunkers::pdf::PdfChunker;
use crate::ingestion::chunkers::image::ImageChunker;
use crate::ingestion::chunkers::text::TextChunker;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

#[pyclass]
#[derive(Debug, Clone)]
pub struct IngestionRouter {
    pub markdown_chunker: MarkdownHeaderChunker,
    pub code_chunker: ASTCodeChunker,
    pub structured_chunker: StructuredDataChunker,
    pub tabular_chunker: TabularChunker,
    pub pdf_chunker: PdfChunker,
    pub image_chunker: ImageChunker,
    pub text_chunker: TextChunker,
}

#[pymethods]
impl IngestionRouter {
    #[new]
    #[pyo3(signature = (max_chunk_chars=1800, overlap_chars=200))]
    pub fn new(max_chunk_chars: usize, overlap_chars: usize) -> Self {
        Self {
            markdown_chunker: MarkdownHeaderChunker::new(max_chunk_chars, overlap_chars),
            code_chunker: ASTCodeChunker::new(max_chunk_chars),
            structured_chunker: StructuredDataChunker::new(max_chunk_chars),
            tabular_chunker: TabularChunker::new(max_chunk_chars),
            pdf_chunker: PdfChunker::new(max_chunk_chars),
            image_chunker: ImageChunker::new(max_chunk_chars),
            text_chunker: TextChunker::new(max_chunk_chars, overlap_chars),
        }
    }

    /// Determines whether the given file path or extension has a specialized Rust chunker available.
    pub fn supports_file(&self, file_path: &str) -> bool {
        if self.text_chunker.is_web_url(file_path) {
            return true;
        }

        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");

        matches!(ext.as_str(), "md" | "markdown" | "rst" | "mdown")
            || self.code_chunker.supports_extension(&ext)
            || self.structured_chunker.supports_extension(&ext)
            || self.tabular_chunker.supports_extension(&ext)
            || self.pdf_chunker.supports_extension(&ext)
            || self.image_chunker.supports_extension(&ext)
            || self.text_chunker.supports_extension(&ext)
            || self.text_chunker.supports_filename(file_name)
    }

    /// Chunks document content using the specialized parser matching the file extension.
    pub fn chunk_text(&self, file_path: &str, content: &str) -> PyResult<Vec<ChunkPayload>> {
        if self.text_chunker.is_web_url(file_path) {
            if content.lines().any(|l| l.trim_start().starts_with('#')) {
                return self.markdown_chunker.chunk(file_path, content).map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Markdown web chunker error: {}", e))
                });
            } else {
                return self.text_chunker.chunk(file_path, content).map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!("Text web chunker error: {}", e))
                });
            }
        }

        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if matches!(ext.as_str(), "md" | "markdown" | "rst" | "mdown") {
            self.markdown_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Markdown chunker error: {}", e))
            })
        } else if self.code_chunker.supports_extension(&ext) {
            self.code_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Code AST chunker error: {}", e))
            })
        } else if self.structured_chunker.supports_extension(&ext) {
            self.structured_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Structured data chunker error: {}", e))
            })
        } else if self.tabular_chunker.supports_extension(&ext) {
            self.tabular_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Tabular chunker error: {}", e))
            })
        } else if self.pdf_chunker.supports_extension(&ext) {
            self.pdf_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("PDF chunker error: {}", e))
            })
        } else if self.text_chunker.supports_extension(&ext) || self.text_chunker.supports_filename(file_name) {
            self.text_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Text chunker error: {}", e))
            })
        } else {
            self.text_chunker.chunk(file_path, content).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Universal text chunker error: {}", e))
            })
        }
    }

    /// Reads and chunks a file directly from the filesystem in high-speed native Rust.
    /// Handles binary spreadsheet files (.xlsx, .xls, .ods), PDFs (.pdf), and images (.png, .jpg, .webp).
    pub fn chunk_file(&self, file_path: &str) -> PyResult<Vec<ChunkPayload>> {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        if self.tabular_chunker.supports_extension(&ext) && matches!(ext.as_str(), "xlsx" | "xls" | "ods") {
            self.tabular_chunker.chunk_file(file_path).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Tabular Excel chunker error: {}", e))
            })
        } else if self.pdf_chunker.supports_extension(&ext) {
            self.pdf_chunker.chunk_file(file_path).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("PDF chunker error: {}", e))
            })
        } else if self.image_chunker.supports_extension(&ext) {
            self.image_chunker.chunk_file(file_path).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Image chunker error: {}", e))
            })
        } else {
            let content = std::fs::read_to_string(file_path).map_err(|e| {
                pyo3::exceptions::PyIOError::new_err(format!("Failed to read file '{}': {}", file_path, e))
            })?;
            self.chunk_text(file_path, &content)
        }
    }

    /// Chunks raw byte content (useful for binary workbooks, PDFs, images, or in-memory streams).
    pub fn chunk_bytes(&self, file_path: &str, bytes: &[u8]) -> PyResult<Vec<ChunkPayload>> {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        if matches!(ext.as_str(), "xlsx" | "xls" | "ods") {
            self.tabular_chunker.excel_chunker.chunk_bytes(file_path, bytes).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Tabular Excel bytes error: {}", e))
            })
        } else if self.pdf_chunker.supports_extension(&ext) {
            self.pdf_chunker.chunk_bytes(file_path, bytes).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("PDF bytes error: {}", e))
            })
        } else if self.image_chunker.supports_extension(&ext) {
            self.image_chunker.chunk_bytes(file_path, bytes).map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Image bytes error: {}", e))
            })
        } else {
            let content = String::from_utf8_lossy(bytes);
            self.chunk_text(file_path, &content)
        }
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
        assert!(router.supports_file("MainActivity.kt"));
        assert!(router.supports_file("build.gradle.kts"));
        assert!(router.supports_file("AppCoordinator.swift"));
        assert!(router.supports_file("model.rb"));
        assert!(router.supports_file("index.php"));
        assert!(router.supports_file("template.phtml"));
        assert!(router.supports_file("init.lua"));
        assert!(router.supports_file("main.dart"));
        assert!(router.supports_file("pom.xml"));
        assert!(router.supports_file("package.json"));
        assert!(router.supports_file("events.jsonl"));
        assert!(router.supports_file("records.ndjson"));
        assert!(router.supports_file("docker-compose.yml"));
        assert!(router.supports_file("manifest.yaml"));
        assert!(router.supports_file("Cargo.toml"));
        assert!(router.supports_file("data.csv"));
        assert!(router.supports_file("catalog.tsv"));
        assert!(router.supports_file("financials.xlsx"));
        assert!(router.supports_file("legacy.xls"));
        assert!(router.supports_file("budget.ods"));
        assert!(router.supports_file("statement.ofx"));
        assert!(router.supports_file("report.pdf"));
        assert!(router.supports_file("image.png"));
        assert!(!router.supports_file("binary.bin"));
        assert!(!router.supports_file("unknown.xyz"));
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

    #[test]
    fn test_router_chunk_kotlin() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "package com.example\n\nclass MainActivity {\n    fun onCreate() {\n        println(\"Created\")\n    }\n}\n";
        let chunks = router.chunk_text("MainActivity.kt", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: MainActivity.kt > class MainActivity")));
    }

    #[test]
    fn test_router_chunk_swift() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "import Foundation\n\nclass AppCoordinator {\n    func start() {\n        print(\"Started\")\n    }\n}\n";
        let chunks = router.chunk_text("AppCoordinator.swift", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: AppCoordinator.swift > class AppCoordinator")));
    }

    #[test]
    fn test_router_chunk_ruby() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "class User\n  def full_name\n    \"#{first_name} #{last_name}\"\n  end\nend\n";
        let chunks = router.chunk_text("user.rb", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: user.rb > class User")));
    }

    #[test]
    fn test_router_chunk_php() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "<?php\n\nclass OrderService {\n    public function createOrder(): int {\n        return 123;\n    }\n}\n";
        let chunks = router.chunk_text("OrderService.php", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: OrderService.php > class OrderService")));
    }

    #[test]
    fn test_router_chunk_lua() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "local M = {}\n\nfunction M.greet(name)\n    return \"Hello, \" .. name\nend\n\nreturn M\n";
        let chunks = router.chunk_text("init.lua", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: init.lua > function M.greet")));
    }

    #[test]
    fn test_router_chunk_dart() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let code = "import 'package:flutter/material.dart';\n\nclass MyApp extends StatelessWidget {\n  Widget build(BuildContext context) {\n    return Container();\n  }\n}\n";
        let chunks = router.chunk_text("main.dart", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: main.dart > class MyApp")));
    }

    #[test]
    fn test_router_chunk_xml() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let xml = "<project><modelVersion>4.0.0</modelVersion><groupId>com.mycompany</groupId></project>";
        let chunks = router.chunk_text("pom.xml", xml).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "xml");
        assert!(chunks[0].text.contains("// Context: pom.xml > project"));
    }

    #[test]
    fn test_router_chunk_json() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let json = r#"{"name": "any-context", "version": "0.30.7", "private": true}"#;
        let chunks = router.chunk_text("package.json", json).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "json");
        assert!(chunks[0].text.contains("// Context: package.json > root"));
    }

    #[test]
    fn test_router_chunk_jsonl() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let jsonl = "{\"id\": 1, \"event\": \"start\"}\n{\"id\": 2, \"event\": \"stop\"}\n";
        let chunks = router.chunk_text("logs.jsonl", jsonl).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "json");
        assert!(chunks[0].text.contains("// Context: logs.jsonl > lines"));
    }

    #[test]
    fn test_router_chunk_yaml() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let yaml = "version: '3.8'\nservices:\n  redis:\n    image: redis:alpine\n";
        let chunks = router.chunk_text("docker-compose.yml", yaml).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "yaml");
        assert!(chunks[0].text.contains("// Context: docker-compose.yml"));
    }

    #[test]
    fn test_router_chunk_toml() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let toml = "[package]\nname = \"any-context\"\nversion = \"0.30.8\"\n";
        let chunks = router.chunk_text("Cargo.toml", toml).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "toml");
        assert!(chunks[0].text.contains("// Context: Cargo.toml > root"));
    }

    #[test]
    fn test_router_chunk_csv() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let csv = "Col1,Col2\nVal1,Val2\n";
        let chunks = router.chunk_text("test.csv", csv).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "csv");
        assert!(chunks[0].text.contains("| Col1 | Col2 |"));
    }

    #[test]
    fn test_router_chunk_tsv() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let tsv = "ID\tProduct\n1\tBook\n";
        let chunks = router.chunk_text("test.tsv", tsv).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "tsv");
        assert!(chunks[0].text.contains("| ID | Product |"));
    }

    #[test]
    fn test_router_chunk_ofx() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let ofx = "<OFX><BANKID>001<ACCTID>123<STMTTRN><TRNTYPE>DEBIT<DTPOSTED>20260901<TRNAMT>-50.00<MEMO>Dinner</STMTTRN></OFX>";
        let chunks = router.chunk_text("extrato.ofx", ofx).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "ofx");
        assert!(chunks[0].text.contains("Bank: 001 | Acct: 123"));
    }

    #[test]
    fn test_router_supports_pdf_and_images() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        assert!(router.supports_file("report.pdf"));
        assert!(router.supports_file("diagram.png"));
        assert!(router.supports_file("photo.jpg"));
        assert!(router.supports_file("photo.jpeg"));
        assert!(router.supports_file("chart.webp"));
    }

    #[test]
    fn test_router_chunk_pdf_text() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let pdf_text = "Chapter 1: Native Rust Core Architecture\nThis document describes the high-performance engine.";
        let chunks = router.chunk_text("manual.pdf", pdf_text).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "pdf");
        assert!(chunks[0].text.contains("// Context: manual.pdf > Content"));
    }

    #[test]
    fn test_router_chunk_image_bytes() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let chunks = router.chunk_bytes("architecture.png", b"fake image bytes").expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "visual_diagram");
        assert!(chunks[0].text.contains("Visual Image & Diagram Specification"));
    }

    #[test]
    fn test_router_supports_text_scripts_and_web() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        assert!(router.supports_file("notes.txt"));
        assert!(router.supports_file("server.log"));
        assert!(router.supports_file("schema.sql"));
        assert!(router.supports_file("build.sh"));
        assert!(router.supports_file("deploy.bash"));
        assert!(router.supports_file("setup.ps1"));
        assert!(router.supports_file("start.bat"));
        assert!(router.supports_file("run.cmd"));
        assert!(router.supports_file(".env"));
        assert!(router.supports_file("config.ini"));
        assert!(router.supports_file("app.properties"));
        assert!(router.supports_file("Dockerfile"));
        assert!(router.supports_file("Makefile"));
        assert!(router.supports_file("https://example.com/docs"));
        assert!(router.supports_file("http://localhost:3000/api"));
    }

    #[test]
    fn test_router_chunk_sql_and_web() {
        pyo3::prepare_freethreaded_python();
        let router = IngestionRouter::new(1800, 200);
        let sql = "CREATE TABLE users (id INT PRIMARY KEY, name TEXT);\nINSERT INTO users VALUES (1, 'Admin');\n";
        let chunks = router.chunk_text("migrations.sql", sql).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "sql");
        assert!(chunks[0].text.contains("// Context: migrations.sql > lines"));

        let web_content = "# API Overview\nThis is the REST API documentation.";
        let web_chunks = router.chunk_text("https://api.acme.org/docs", web_content).expect("Chunking failed");
        assert!(!web_chunks.is_empty());
        assert_eq!(web_chunks[0].content_type, "markdown");
    }
}
