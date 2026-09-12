pub mod csharp;
pub mod java;
pub mod python;
pub mod traits;
pub mod typescript;

use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;
use std::path::Path;
use traits::LanguageASTParser;

#[derive(Debug, Clone)]
pub struct ASTCodeChunker {
    python_parser: python::PythonASTParser,
    typescript_parser: typescript::TypeScriptASTParser,
    java_parser: java::JavaASTParser,
    csharp_parser: csharp::CSharpASTParser,
    max_chunk_chars: usize,
}

impl ASTCodeChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            python_parser: python::PythonASTParser::new(),
            typescript_parser: typescript::TypeScriptASTParser::new(),
            java_parser: java::JavaASTParser::new(),
            csharp_parser: csharp::CSharpASTParser::new(),
            max_chunk_chars,
        }
    }

    pub fn supports_extension(&self, ext: &str) -> bool {
        matches!(
            ext.to_lowercase().as_str(),
            "py" | "pyw" | "pyi"
                | "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs"
                | "java"
                | "cs"
        )
    }
}

impl Chunker for ASTCodeChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext.to_lowercase().as_str() {
            "py" | "pyw" | "pyi" => self.python_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" => {
                self.typescript_parser.parse_chunks(file_path, content, self.max_chunk_chars)
            }
            "java" => self.java_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "cs" => self.csharp_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            _ => Err(format!("Unsupported code extension for AST parsing: .{}", ext)),
        }
    }
}
