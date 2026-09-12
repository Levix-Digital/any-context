pub mod cpp;
pub mod csharp;
pub mod dart;
pub mod go;
pub mod java;
pub mod kotlin;
pub mod lua;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod splitter;
pub mod swift;
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
    go_parser: go::GoASTParser,
    rust_parser: rust::RustASTParser,
    cpp_parser: cpp::CppASTParser,
    kotlin_parser: kotlin::KotlinASTParser,
    swift_parser: swift::SwiftASTParser,
    ruby_parser: ruby::RubyASTParser,
    php_parser: php::PhpASTParser,
    lua_parser: lua::LuaASTParser,
    dart_parser: dart::DartASTParser,
    max_chunk_chars: usize,
}

impl ASTCodeChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            python_parser: python::PythonASTParser::new(),
            typescript_parser: typescript::TypeScriptASTParser::new(),
            java_parser: java::JavaASTParser::new(),
            csharp_parser: csharp::CSharpASTParser::new(),
            go_parser: go::GoASTParser::new(),
            rust_parser: rust::RustASTParser::new(),
            cpp_parser: cpp::CppASTParser::new(),
            kotlin_parser: kotlin::KotlinASTParser::new(),
            swift_parser: swift::SwiftASTParser::new(),
            ruby_parser: ruby::RubyASTParser::new(),
            php_parser: php::PhpASTParser::new(),
            lua_parser: lua::LuaASTParser::new(),
            dart_parser: dart::DartASTParser::new(),
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
                | "go"
                | "rs"
                | "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "c++" | "hh" | "hxx"
                | "kt" | "kts"
                | "swift"
                | "rb"
                | "php" | "phtml"
                | "lua"
                | "dart"
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
            "go" => self.go_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "rs" => self.rust_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "c++" | "hh" | "hxx" => {
                self.cpp_parser.parse_chunks(file_path, content, self.max_chunk_chars)
            }
            "kt" | "kts" => self.kotlin_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "swift" => self.swift_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "rb" => self.ruby_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "php" | "phtml" => self.php_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "lua" => self.lua_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            "dart" => self.dart_parser.parse_chunks(file_path, content, self.max_chunk_chars),
            _ => Err(format!("Unsupported code extension for AST parsing: .{}", ext)),
        }
    }
}
