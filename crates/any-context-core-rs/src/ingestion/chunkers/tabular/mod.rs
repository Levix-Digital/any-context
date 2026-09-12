pub mod csv;
pub mod excel;
pub mod ofx;

use std::path::Path;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

pub use self::csv::CsvChunker;
pub use excel::ExcelChunker;
pub use ofx::OfxChunker;

#[derive(Debug, Clone)]
pub struct TabularChunker {
    pub csv_chunker: CsvChunker,
    pub excel_chunker: ExcelChunker,
    pub ofx_chunker: OfxChunker,
    pub max_chunk_chars: usize,
}

impl TabularChunker {
    pub fn new(max_chars: usize) -> Self {
        Self {
            csv_chunker: CsvChunker::new(max_chars),
            excel_chunker: ExcelChunker::new(max_chars),
            ofx_chunker: OfxChunker::new(max_chars),
            max_chunk_chars: max_chars,
        }
    }

    pub fn supports_extension(&self, ext: &str) -> bool {
        matches!(ext, "csv" | "tsv" | "xlsx" | "xls" | "ods" | "ofx")
    }

    pub fn chunk_file(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        if matches!(ext.as_str(), "xlsx" | "xls" | "ods") {
            self.excel_chunker.chunk_file(file_path)
        } else {
            let content = std::fs::read_to_string(file_path)
                .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;
            self.chunk(file_path, &content)
        }
    }
}

impl Chunker for TabularChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

        match ext.as_str() {
            "csv" | "tsv" => self.csv_chunker.chunk(file_path, content),
            "ofx" => self.ofx_chunker.chunk(file_path, content),
            "xlsx" | "xls" | "ods" => {
                // If content is empty but file exists on disk, parse directly
                if p.exists() {
                    self.excel_chunker.chunk_file(file_path)
                } else {
                    Err(format!("Binary spreadsheet format '{}' requires a valid file path on disk", ext))
                }
            }
            _ => Err(format!("Unsupported tabular extension: '{}'", ext)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_extensions() {
        let chunker = TabularChunker::new(1800);
        assert!(chunker.supports_extension("csv"));
        assert!(chunker.supports_extension("tsv"));
        assert!(chunker.supports_extension("xlsx"));
        assert!(chunker.supports_extension("xls"));
        assert!(chunker.supports_extension("ods"));
        assert!(chunker.supports_extension("ofx"));
        assert!(!chunker.supports_extension("txt"));
        assert!(!chunker.supports_extension("json"));
    }

    #[test]
    fn test_delegation_csv() {
        let chunker = TabularChunker::new(1800);
        let csv = "Col1,Col2\nVal1,Val2\n";
        let chunks = chunker.chunk("test.csv", csv).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "csv");
    }

    #[test]
    fn test_delegation_tsv() {
        let chunker = TabularChunker::new(1800);
        let tsv = "Col1\tCol2\nVal1\tVal2\n";
        let chunks = chunker.chunk("test.tsv", tsv).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "tsv");
    }

    #[test]
    fn test_delegation_ofx() {
        let chunker = TabularChunker::new(1800);
        let ofx = "<OFX><BANKID>123<ACCTID>456<STMTTRN><TRNTYPE>DEBIT<DTPOSTED>20260901<TRNAMT>-10<MEMO>Test</STMTTRN></OFX>";
        let chunks = chunker.chunk("bank.ofx", ofx).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "ofx");
    }
}
