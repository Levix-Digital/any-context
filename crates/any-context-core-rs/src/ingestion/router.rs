use std::path::Path;
use crate::ingestion::chunkers::code::ASTCodeChunker;
use crate::ingestion::chunkers::markdown::MarkdownHeaderChunker;
use crate::ingestion::chunkers::structured::StructuredDataChunker;
use crate::ingestion::chunkers::tabular::TabularChunker;
use crate::ingestion::chunkers::pdf::PdfChunker;
use crate::ingestion::chunkers::image::ImageChunker;
use crate::ingestion::chunkers::text::TextChunker;
use crate::ingestion::chunkers::office::OfficeChunker;
use crate::ingestion::chunkers::spatial_form::SpatialFormChunker;
use crate::ingestion::secrets::redact_secrets;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

/// Chunks of one document with its secret values already redacted (BUG-024, ADR-0149).
#[derive(Debug, Clone, Default)]
pub struct RedactedChunks {
    pub chunks: Vec<ChunkPayload>,
    /// Secret values replaced by `[SECRET REDACTED]` in this document.
    pub secrets_redacted: usize,
}

#[derive(Debug, Clone)]
pub struct IngestionRouter {
    pub markdown_chunker: MarkdownHeaderChunker,
    pub code_chunker: ASTCodeChunker,
    pub structured_chunker: StructuredDataChunker,
    pub tabular_chunker: TabularChunker,
    pub pdf_chunker: PdfChunker,
    pub image_chunker: ImageChunker,
    pub text_chunker: TextChunker,
    pub office_chunker: OfficeChunker,
    pub spatial_chunker: SpatialFormChunker,
}

impl IngestionRouter {
    pub fn new(max_chunk_chars: usize, overlap_chars: usize) -> Self {
        Self {
            markdown_chunker: MarkdownHeaderChunker::new(max_chunk_chars, overlap_chars),
            code_chunker: ASTCodeChunker::new(max_chunk_chars),
            structured_chunker: StructuredDataChunker::new(max_chunk_chars),
            tabular_chunker: TabularChunker::new(max_chunk_chars),
            pdf_chunker: PdfChunker::new(max_chunk_chars.max(8000)),
            image_chunker: ImageChunker::new(max_chunk_chars),
            text_chunker: TextChunker::new(max_chunk_chars, overlap_chars),
            office_chunker: OfficeChunker::new(max_chunk_chars, overlap_chars),
            spatial_chunker: SpatialFormChunker::new(max_chunk_chars),
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
            || self.office_chunker.supports_extension(&ext)
            || self.image_chunker.supports_extension(&ext)
            || self.text_chunker.supports_extension(&ext)
            || self.text_chunker.supports_filename(file_name)
    }

    /// Chunks document content using the specialized parser matching the file extension.
    pub fn chunk_text(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        self.chunk_text_native(file_path, content)
    }

    /// Reads and chunks a file directly from the filesystem in high-speed native Rust.
    /// Handles binary spreadsheet files (.xlsx, .xls, .ods), PDFs (.pdf), and images (.png, .jpg, .webp).
    pub fn chunk_file(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        self.chunk_file_native(file_path)
    }

    /// Chunks raw byte content (useful for binary workbooks, PDFs, images, or in-memory streams).
    pub fn chunk_bytes(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        self.chunk_bytes_native(file_path, bytes)
    }
}

impl Default for IngestionRouter {
    fn default() -> Self {
        Self::new(1800, 200)
    }
}

impl IngestionRouter {
    /// Whether the chunker chosen for `file_path` reports file line numbers in
    /// `start_line`/`end_line` (ADR-0142). Markdown, code, structured data and plain text do;
    /// web pages (lines of the fetched text), PDF and Office (page or extracted-text lines),
    /// images (OCR lines) and tabular files (row or transaction indexes) do not, and keep
    /// their page, sheet or row locator in `header_path`.
    pub fn has_file_lines(&self, file_path: &str) -> bool {
        if self.text_chunker.is_web_url(file_path) {
            return false;
        }
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if matches!(ext.as_str(), "md" | "markdown" | "rst" | "mdown")
            || self.code_chunker.supports_extension(&ext)
            || self.structured_chunker.supports_extension(&ext)
        {
            return true;
        }
        !(self.tabular_chunker.supports_extension(&ext)
            || self.pdf_chunker.supports_extension(&ext)
            || self.office_chunker.supports_extension(&ext)
            || self.image_chunker.supports_extension(&ext))
    }

    /// Pure Rust native implementation for document chunking. Secret values are redacted.
    pub fn chunk_text_native(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        self.chunk_text_redacted(file_path, content).map(|r| r.chunks)
    }

    /// Pure Rust native implementation for direct filesystem file chunking. Secret values are redacted.
    pub fn chunk_file_native(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        self.chunk_file_redacted(file_path).map(|r| r.chunks)
    }

    /// Pure Rust native implementation for raw byte chunking. Secret values are redacted.
    pub fn chunk_bytes_native(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        self.chunk_bytes_redacted(file_path, bytes).map(|r| r.chunks)
    }

    /// Chunks text content after redacting its secrets. The whole text is redacted before
    /// chunking, so a private key block is never split across chunks and line numbers stay.
    pub fn chunk_text_redacted(&self, file_path: &str, content: &str) -> Result<RedactedChunks, String> {
        let redaction = redact_secrets(content);
        let chunks = self.dispatch_text(file_path, &redaction.text)?;
        Ok(RedactedChunks { chunks, secrets_redacted: redaction.secrets_found })
    }

    /// Reads and chunks a file with its secrets redacted: text formats before chunking,
    /// extracted formats (PDF, Office, workbooks, images) on each chunk after extraction.
    pub fn chunk_file_redacted(&self, file_path: &str) -> Result<RedactedChunks, String> {
        if self.is_extracted_format(file_path) {
            return self.dispatch_extracted_file(file_path).map(redact_chunks);
        }
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
        self.chunk_text_redacted(file_path, &content)
    }

    /// Chunks raw bytes with their secrets redacted (see `chunk_file_redacted`).
    pub fn chunk_bytes_redacted(&self, file_path: &str, bytes: &[u8]) -> Result<RedactedChunks, String> {
        if self.is_extracted_format(file_path) {
            return self.dispatch_extracted_bytes(file_path, bytes).map(redact_chunks);
        }
        let content = String::from_utf8_lossy(bytes);
        self.chunk_text_redacted(file_path, &content)
    }

    /// Formats whose text only exists after extraction inside their chunker.
    fn is_extracted_format(&self, file_path: &str) -> bool {
        let ext = Path::new(file_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        matches!(ext.as_str(), "xlsx" | "xls" | "ods")
            || self.pdf_chunker.supports_extension(&ext)
            || self.office_chunker.supports_extension(&ext)
            || self.image_chunker.supports_extension(&ext)
    }

    fn dispatch_text(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        if self.text_chunker.is_web_url(file_path) {
            if content.lines().any(|l| l.trim_start().starts_with('#')) {
                return self.markdown_chunker.chunk(file_path, content);
            } else {
                return self.text_chunker.chunk(file_path, content);
            }
        }

        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if matches!(ext.as_str(), "md" | "markdown" | "rst" | "mdown") {
            self.markdown_chunker.chunk(file_path, content)
        } else if self.code_chunker.supports_extension(&ext) {
            self.code_chunker.chunk(file_path, content)
        } else if self.structured_chunker.supports_extension(&ext) {
            self.structured_chunker.chunk(file_path, content)
        } else if self.tabular_chunker.supports_extension(&ext) {
            self.tabular_chunker.chunk(file_path, content)
        } else if self.pdf_chunker.supports_extension(&ext) {
            self.pdf_chunker.chunk(file_path, content)
        } else if self.text_chunker.supports_extension(&ext) || self.text_chunker.supports_filename(file_name) {
            self.text_chunker.chunk(file_path, content)
        } else {
            self.text_chunker.chunk(file_path, content)
        }
    }

    fn dispatch_extracted_file(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        let ext = Path::new(file_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if matches!(ext.as_str(), "xlsx" | "xls" | "ods") {
            self.tabular_chunker.chunk_file(file_path)
        } else if self.pdf_chunker.supports_extension(&ext) {
            self.pdf_chunker.chunk_file(file_path)
        } else if self.office_chunker.supports_extension(&ext) {
            self.office_chunker.chunk_file(file_path)
        } else {
            self.image_chunker.chunk_file(file_path)
        }
    }

    fn dispatch_extracted_bytes(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        let ext = Path::new(file_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if matches!(ext.as_str(), "xlsx" | "xls" | "ods") {
            self.tabular_chunker.excel_chunker.chunk_bytes(file_path, bytes)
        } else if self.pdf_chunker.supports_extension(&ext) {
            self.pdf_chunker.chunk_bytes(file_path, bytes)
        } else if self.office_chunker.supports_extension(&ext) {
            self.office_chunker.chunk_bytes(file_path, bytes)
        } else {
            self.image_chunker.chunk_bytes(file_path, bytes)
        }
    }
}

/// Redacts the text and section path of chunks produced by an extracting chunker.
fn redact_chunks(mut chunks: Vec<ChunkPayload>) -> RedactedChunks {
    let mut secrets_redacted = 0;
    for chunk in &mut chunks {
        let text = redact_secrets(&chunk.text);
        secrets_redacted += text.secrets_found;
        chunk.text = text.text;
        if let Some(header) = chunk.header_path.as_deref() {
            let header = redact_secrets(header);
            secrets_redacted += header.secrets_found;
            chunk.header_path = Some(header.text);
        }
    }
    RedactedChunks { chunks, secrets_redacted }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Token-shaped sample built at run time, so no complete credential literal is committed.
    fn fake_secret(prefix: &str, len: usize, seed: u64) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let body: String = (0..len)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ALPHABET[((state >> 33) as usize) % ALPHABET.len()] as char
            })
            .collect();
        format!("{prefix}{body}")
    }

    fn joined(chunks: &[ChunkPayload]) -> String {
        chunks.iter().map(|c| format!("{}\n{}", c.text, c.header_path.clone().unwrap_or_default())).collect()
    }

    #[test]
    fn test_secret_files_are_chunked_with_their_values_redacted() {
        let router = IngestionRouter::default();
        let secret = fake_secret("", 24, 41);
        let token = fake_secret(&["gh", "p_"].concat(), 36, 42);
        let pem = format!(
            "{}\n{secret}\n{}\n",
            ["-----BEGIN", " EC PRIVATE KEY-----"].concat(),
            ["-----END", " EC PRIVATE KEY-----"].concat()
        );
        let cases = [
            ("/repo/.env", format!("APP_NAME=billing\nDB_PASSWORD={secret}\nGITHUB_TOKEN={token}\n"), "DB_PASSWORD"),
            ("/repo/.env.production", format!("API_KEY={secret}\n"), "API_KEY"),
            ("/repo/infra/terraform.tfvars", format!("region = \"eu-west-1\"\ndb_password = \"{secret}\"\n"), "db_password"),
            ("/repo/certs/server.key", pem.clone(), "[SECRET REDACTED]"),
            ("/repo/config.yaml", format!("service:\n  name: billing\n  client_secret: {secret}\n"), "client_secret"),
            ("/repo/settings.json", format!("{{\"apiKey\": \"{secret}\", \"region\": \"eu\"}}"), "apiKey"),
            ("/repo/src/config.py", format!("GITHUB = \"{token}\"\n"), "GITHUB"),
        ];
        for (path, content, kept) in &cases {
            let r = router.chunk_text_redacted(path, content).expect("chunk");
            let text = joined(&r.chunks);
            assert!(!text.contains(&secret) && !text.contains(&token), "{path}: {text}");
            assert!(text.contains(kept), "{path} loses '{kept}': {text}");
            assert!(r.secrets_redacted >= 1, "{path}");
        }
    }

    #[test]
    fn test_private_key_file_on_disk_is_redacted() {
        let dir = std::env::temp_dir().join(format!("actx_router_pem_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("server.key");
        let body = fake_secret("MIIEvQIBADANBgkqhkiG9w0BAQEFAASC", 30, 43);
        std::fs::write(&path, format!("{}\n{body}\n{}\n", ["-----BEGIN", " PRIVATE KEY-----"].concat(), ["-----END", " PRIVATE KEY-----"].concat())).unwrap();
        let r = IngestionRouter::default().chunk_file_redacted(&path.to_string_lossy()).expect("chunk");
        assert_eq!(r.secrets_redacted, 1);
        assert!(!joined(&r.chunks).contains(&body));
        assert!(!IngestionRouter::default().chunk_file_native(&path.to_string_lossy()).map(|c| joined(&c)).unwrap().contains(&body));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_extracted_chunks_are_redacted_after_extraction() {
        let secret = fake_secret("", 20, 44);
        let chunk = ChunkPayload::new(
            "id".to_string(),
            format!("Page 2\npassword: {secret}\nThe rest of the page."),
            "manual.pdf".to_string(),
            "/docs/manual.pdf".to_string(),
            Some(format!("Setup > token={secret}")),
            0,
            0,
            "pdf".to_string(),
            0,
        );
        let r = redact_chunks(vec![chunk]);
        assert_eq!(r.secrets_redacted, 2);
        assert!(!joined(&r.chunks).contains(&secret));
        assert!(r.chunks[0].text.contains("The rest of the page."));
        let router = IngestionRouter::default();
        assert!(router.is_extracted_format("/docs/manual.pdf"));
        assert!(router.is_extracted_format("/docs/book.xlsx"));
        assert!(!router.is_extracted_format("/repo/.env"));
        assert!(!router.is_extracted_format("/repo/server.key"));
    }

    #[test]
    fn test_has_file_lines() {
        let router = IngestionRouter::default();
        for path in ["docs/guide.md", "src/client.rs", "config/app.json", "notes.txt", "Dockerfile"] {
            assert!(router.has_file_lines(path), "{path} has file lines");
        }
        for path in [
            "report.pdf",
            "letter.docx",
            "scan.png",
            "data.csv",
            "budget.xlsx",
            "bank.ofx",
            "https://docs.example.com/guide.md",
        ] {
            assert!(!router.has_file_lines(path), "{path} has no file lines");
        }
    }

    #[test]
    fn test_router_supports_files() {
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

    #[test]
    fn test_router_chunk_typescript() {
        let router = IngestionRouter::new(1800, 200);
        let code = "export function calculateTotal(items: number[]): number {\n  return items.reduce((a, b) => a + b, 0);\n}\n";
        let chunks = router.chunk_text("math.ts", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: math.ts > function calculateTotal")));
    }

    #[test]
    fn test_router_chunk_java() {
        let router = IngestionRouter::new(1800, 200);
        let code = "public class Calculator {\n    public int add(int a, int b) {\n        return a + b;\n    }\n}\n";
        let chunks = router.chunk_text("Calculator.java", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: Calculator.java > class Calculator")));
    }

    #[test]
    fn test_router_chunk_csharp() {
        let router = IngestionRouter::new(1800, 200);
        let code = "namespace MyStore;\npublic class OrderProcessor {\n    public void Process() {}\n}\n";
        let chunks = router.chunk_text("OrderProcessor.cs", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: OrderProcessor.cs > class OrderProcessor")));
    }

    #[test]
    fn test_router_chunk_go() {
        let router = IngestionRouter::new(1800, 200);
        let code = "package main\n\nfunc RunApp() error {\n    return nil\n}\n";
        let chunks = router.chunk_text("main.go", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: main.go > function RunApp")));
    }

    #[test]
    fn test_router_chunk_rust() {
        let router = IngestionRouter::new(1800, 200);
        let code = "pub fn execute_task() -> bool {\n    true\n}\n";
        let chunks = router.chunk_text("task.rs", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: task.rs > fn execute_task")));
    }

    #[test]
    fn test_router_chunk_cpp() {
        let router = IngestionRouter::new(1800, 200);
        let code = "#include <iostream>\n\nclass Engine {\npublic:\n    void start();\n};\n";
        let chunks = router.chunk_text("Engine.cpp", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: Engine.cpp > class Engine")));
    }

    #[test]
    fn test_router_chunk_kotlin() {
        let router = IngestionRouter::new(1800, 200);
        let code = "package com.example\n\nclass MainActivity {\n    fun onCreate() {\n        println(\"Created\")\n    }\n}\n";
        let chunks = router.chunk_text("MainActivity.kt", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: MainActivity.kt > class MainActivity")));
    }

    #[test]
    fn test_router_chunk_swift() {
        let router = IngestionRouter::new(1800, 200);
        let code = "import Foundation\n\nclass AppCoordinator {\n    func start() {\n        print(\"Started\")\n    }\n}\n";
        let chunks = router.chunk_text("AppCoordinator.swift", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: AppCoordinator.swift > class AppCoordinator")));
    }

    #[test]
    fn test_router_chunk_ruby() {
        let router = IngestionRouter::new(1800, 200);
        let code = "class User\n  def full_name\n    \"#{first_name} #{last_name}\"\n  end\nend\n";
        let chunks = router.chunk_text("user.rb", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: user.rb > class User")));
    }

    #[test]
    fn test_router_chunk_php() {
        let router = IngestionRouter::new(1800, 200);
        let code = "<?php\n\nclass OrderService {\n    public function createOrder(): int {\n        return 123;\n    }\n}\n";
        let chunks = router.chunk_text("OrderService.php", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: OrderService.php > class OrderService")));
    }

    #[test]
    fn test_router_chunk_lua() {
        let router = IngestionRouter::new(1800, 200);
        let code = "local M = {}\n\nfunction M.greet(name)\n    return \"Hello, \" .. name\nend\n\nreturn M\n";
        let chunks = router.chunk_text("init.lua", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: init.lua > function M.greet")));
    }

    #[test]
    fn test_router_chunk_dart() {
        let router = IngestionRouter::new(1800, 200);
        let code = "import 'package:flutter/material.dart';\n\nclass MyApp extends StatelessWidget {\n  Widget build(BuildContext context) {\n    return Container();\n  }\n}\n";
        let chunks = router.chunk_text("main.dart", code).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("// Context: main.dart > class MyApp")));
    }

    #[test]
    fn test_router_chunk_xml() {
        let router = IngestionRouter::new(1800, 200);
        let xml = "<project><modelVersion>4.0.0</modelVersion><groupId>com.mycompany</groupId></project>";
        let chunks = router.chunk_text("pom.xml", xml).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "xml");
        assert!(chunks[0].text.contains("// Context: pom.xml > project"));
    }

    #[test]
    fn test_router_chunk_json() {
        let router = IngestionRouter::new(1800, 200);
        let json = r#"{"name": "any-context", "version": "0.30.7", "private": true}"#;
        let chunks = router.chunk_text("package.json", json).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "json");
        assert!(chunks[0].text.contains("// Context: package.json > root"));
    }

    #[test]
    fn test_router_chunk_jsonl() {
        let router = IngestionRouter::new(1800, 200);
        let jsonl = "{\"id\": 1, \"event\": \"start\"}\n{\"id\": 2, \"event\": \"stop\"}\n";
        let chunks = router.chunk_text("logs.jsonl", jsonl).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "json");
        assert!(chunks[0].text.contains("// Context: logs.jsonl > lines"));
    }

    #[test]
    fn test_router_chunk_yaml() {
        let router = IngestionRouter::new(1800, 200);
        let yaml = "version: '3.8'\nservices:\n  redis:\n    image: redis:alpine\n";
        let chunks = router.chunk_text("docker-compose.yml", yaml).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "yaml");
        assert!(chunks[0].text.contains("// Context: docker-compose.yml"));
    }

    #[test]
    fn test_router_chunk_toml() {
        let router = IngestionRouter::new(1800, 200);
        let toml = "[package]\nname = \"any-context\"\nversion = \"0.30.8\"\n";
        let chunks = router.chunk_text("Cargo.toml", toml).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "toml");
        assert!(chunks[0].text.contains("// Context: Cargo.toml > root"));
    }

    #[test]
    fn test_router_chunk_csv() {
        let router = IngestionRouter::new(1800, 200);
        let csv = "Col1,Col2\nVal1,Val2\n";
        let chunks = router.chunk_text("test.csv", csv).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "csv");
        assert!(chunks[0].text.contains("| Col1 | Col2 |"));
    }

    #[test]
    fn test_router_chunk_tsv() {
        let router = IngestionRouter::new(1800, 200);
        let tsv = "ID\tProduct\n1\tBook\n";
        let chunks = router.chunk_text("test.tsv", tsv).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "tsv");
        assert!(chunks[0].text.contains("| ID | Product |"));
    }

    #[test]
    fn test_router_chunk_ofx() {
        let router = IngestionRouter::new(1800, 200);
        let ofx = "<OFX><BANKID>001<ACCTID>123<STMTTRN><TRNTYPE>DEBIT<DTPOSTED>20260901<TRNAMT>-50.00<MEMO>Dinner</STMTTRN></OFX>";
        let chunks = router.chunk_text("extrato.ofx", ofx).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "ofx");
        assert!(chunks[0].text.contains("Bank: 001 | Acct: 123"));
    }

    #[test]
    fn test_router_supports_pdf_and_images() {
        let router = IngestionRouter::new(1800, 200);
        assert!(router.supports_file("report.pdf"));
        assert!(router.supports_file("diagram.png"));
        assert!(router.supports_file("photo.jpg"));
        assert!(router.supports_file("photo.jpeg"));
        assert!(router.supports_file("chart.webp"));
    }

    #[test]
    fn test_router_chunk_pdf_text() {
        let router = IngestionRouter::new(1800, 200);
        let pdf_text = "Chapter 1: Native Rust Core Architecture\nThis document describes the high-performance engine.";
        let chunks = router.chunk_text("manual.pdf", pdf_text).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "pdf");
        assert!(chunks[0].text.contains("// Context: manual.pdf > Content"));
    }

    #[test]
    fn test_router_chunk_image_bytes() {
        let router = IngestionRouter::new(1800, 200);
        let chunks = router.chunk_bytes("architecture.png", b"fake image bytes").expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "visual_diagram");
        assert!(chunks[0].text.contains("Visual Image & Diagram Specification"));
    }

    #[test]
    fn test_router_supports_text_scripts_and_web() {
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

    #[test]
    fn test_router_supports_office_docx_and_pptx() {
        let router = IngestionRouter::new(1800, 200);
        assert!(router.supports_file("spec.docx"));
        assert!(router.supports_file("slides.pptx"));
        assert!(router.supports_file("legacy.doc"));
        assert!(router.supports_file("presentation.ppt"));
    }
}
