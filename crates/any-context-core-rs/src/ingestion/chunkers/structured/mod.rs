pub mod xml;
pub mod json;
pub mod yaml;

use std::path::Path;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;
pub use xml::XmlChunker;
pub use json::JsonChunker;
pub use yaml::YamlChunker;

#[derive(Debug, Clone)]
pub struct StructuredDataChunker {
    pub xml_chunker: XmlChunker,
    pub json_chunker: JsonChunker,
    pub yaml_chunker: YamlChunker,
    pub max_chunk_chars: usize,
}

impl StructuredDataChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        let max_chars = if max_chunk_chars == 0 { 1800 } else { max_chunk_chars };
        Self {
            xml_chunker: XmlChunker::new(max_chars),
            json_chunker: JsonChunker::new(max_chars),
            yaml_chunker: YamlChunker::new(max_chars),
            max_chunk_chars: max_chars,
        }
    }

    pub fn supports_extension(&self, ext: &str) -> bool {
        matches!(ext, "xml" | "json" | "jsonl" | "ndjson" | "yaml" | "yml")
    }
}

impl Chunker for StructuredDataChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        match ext.as_str() {
            "xml" => self.xml_chunker.chunk(file_path, content),
            "json" | "jsonl" | "ndjson" => self.json_chunker.chunk(file_path, content),
            "yaml" | "yml" => self.yaml_chunker.chunk(file_path, content),
            _ => Err(format!("Unsupported structured data extension: '{}'", ext)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_extensions() {
        let chunker = StructuredDataChunker::new(1800);
        assert!(chunker.supports_extension("xml"));
        assert!(chunker.supports_extension("json"));
        assert!(chunker.supports_extension("jsonl"));
        assert!(chunker.supports_extension("ndjson"));
        assert!(chunker.supports_extension("yaml"));
        assert!(chunker.supports_extension("yml"));
        assert!(!chunker.supports_extension("txt"));
        assert!(!chunker.supports_extension("py"));
    }

    #[test]
    fn test_delegation_xml() {
        let chunker = StructuredDataChunker::new(1800);
        let xml = "<root><item>Hello</item></root>";
        let chunks = chunker.chunk("test.xml", xml).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "xml");
    }

    #[test]
    fn test_delegation_json() {
        let chunker = StructuredDataChunker::new(1800);
        let json = r#"{"hello": "world"}"#;
        let chunks = chunker.chunk("test.json", json).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "json");
    }

    #[test]
    fn test_delegation_yaml() {
        let chunker = StructuredDataChunker::new(1800);
        let yaml = "hello: world\n";
        let chunks = chunker.chunk("test.yaml", yaml).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "yaml");
    }
}
