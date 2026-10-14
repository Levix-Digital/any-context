use std::io::{Cursor, Read};
use std::path::Path;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use zip::ZipArchive;

use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

/// Universal Native Microsoft Office / OpenXML Document Chunker (.docx, .pptx).
/// Decompresses ZIP archives in memory, parses underlying OpenXML streams (word/document.xml,
/// ppt/slides/slide*.xml) with high-speed zero-alloc SAX parsing, and generates structured
/// semantic chunks with contextual breadcrumbs.
#[derive(Debug, Clone)]
pub struct OfficeChunker {
    pub max_chunk_chars: usize,
    pub overlap_chars: usize,
}

impl Default for OfficeChunker {
    fn default() -> Self {
        Self::new(1800, 200)
    }
}

impl OfficeChunker {
    pub fn new(max_chunk_chars: usize, overlap_chars: usize) -> Self {
        Self {
            max_chunk_chars: if max_chunk_chars == 0 { 1800 } else { max_chunk_chars },
            overlap_chars,
        }
    }

    /// Checks if the extension corresponds to an OpenXML document.
    pub fn supports_extension(&self, ext: &str) -> bool {
        matches!(ext.to_lowercase().as_str(), "docx" | "pptx" | "doc" | "ppt")
    }

    /// Reads and chunks an Office document directly from disk.
    pub fn chunk_file(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        let bytes = std::fs::read(file_path)
            .map_err(|e| format!("Failed to read Office file '{}': {}", file_path, e))?;
        self.chunk_bytes(file_path, &bytes)
    }

    /// Chunks an Office document from in-memory bytes.
    pub fn chunk_bytes(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("document").to_string();

        let (extracted_text, content_type) = if ext == "docx" || ext == "doc" {
            (self.extract_docx_text(bytes)?, "Word Document")
        } else if ext == "pptx" || ext == "ppt" {
            (self.extract_pptx_text(bytes)?, "PowerPoint Presentation")
        } else {
            return Err(format!("Unsupported Office file extension: .{}", ext));
        };

        if extracted_text.trim().is_empty() {
            let fallback_text = format!("// Context: {} > [Empty Office Document]\n\n[Document contains no extractable text]", file_name);
            return Ok(vec![ChunkPayload {
                id: format!("{}_office_0", file_name),
                text: fallback_text,
                file_name: file_name.clone(),
                file_path: file_path.to_string(),
                header_path: Some(format!("{} > [Empty]", file_name)),
                start_line: 1,
                end_line: 1,
                content_type: content_type.to_string(),
                chunk_index: 0,
            }]);
        }

        self.create_chunks(file_path, &file_name, &extracted_text, content_type)
    }

    /// Extracts clean textual content from a DOCX file by parsing `word/document.xml`.
    pub fn extract_docx_text(&self, bytes: &[u8]) -> Result<String, String> {
        let cursor = Cursor::new(bytes);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(e) => return Err(format!("Failed to open DOCX archive: {}", e)),
        };

        let mut xml_content = String::new();
        if let Ok(mut doc_file) = archive.by_name("word/document.xml") {
            let _ = doc_file.read_to_string(&mut xml_content);
        } else {
            return Err("Invalid DOCX format: 'word/document.xml' not found in archive".to_string());
        }

        let mut reader = Reader::from_str(&xml_content);
        reader.config_mut().trim_text(false);

        let mut output = String::new();
        let mut in_text = false;
        let mut current_para = String::new();
        let mut in_table_cell = false;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let local_name = e.local_name();
                    if local_name.as_ref() == b"t" {
                        in_text = true;
                    } else if local_name.as_ref() == b"tc" {
                        in_table_cell = true;
                    }
                }
                Ok(Event::End(ref e)) => {
                    let local_name = e.local_name();
                    if local_name.as_ref() == b"t" {
                        in_text = false;
                    } else if local_name.as_ref() == b"tc" {
                        in_table_cell = false;
                        current_para.push_str(" | ");
                    } else if local_name.as_ref() == b"p" || local_name.as_ref() == b"tr" {
                        let trimmed = current_para.trim();
                        if !trimmed.is_empty() {
                            output.push_str(trimmed);
                            output.push_str("\n\n");
                        }
                        current_para.clear();
                    }
                }
                Ok(Event::Text(ref e)) => {
                    if in_text || in_table_cell {
                        if let Ok(txt) = e.unescape() {
                            current_para.push_str(&txt);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Error parsing Word XML: {}", e)),
                _ => {}
            }
            buf.clear();
        }

        let remaining = current_para.trim();
        if !remaining.is_empty() {
            output.push_str(remaining);
            output.push('\n');
        }

        Ok(output.trim().to_string())
    }

    /// Extracts textual content from a PPTX file by enumerating and ordering slide XMLs.
    pub fn extract_pptx_text(&self, bytes: &[u8]) -> Result<String, String> {
        let cursor = Cursor::new(bytes);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(e) => return Err(format!("Failed to open PPTX archive: {}", e)),
        };

        // Collect all slide entry names
        let mut slide_names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
            .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
            .collect();

        // Sort slide names numerically: slide1.xml, slide2.xml, slide10.xml
        slide_names.sort_by_key(|name| {
            name.strip_prefix("ppt/slides/slide")
                .and_then(|s| s.strip_suffix(".xml"))
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0)
        });

        if slide_names.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();

        for (idx, slide_name) in slide_names.iter().enumerate() {
            let mut xml_content = String::new();
            if let Ok(mut sf) = archive.by_name(slide_name) {
                let _ = sf.read_to_string(&mut xml_content);
            } else {
                continue;
            }

            let slide_num = idx + 1;
            output.push_str(&format!("--- Slide {} ---\n", slide_num));

            let mut reader = Reader::from_str(&xml_content);
            reader.config_mut().trim_text(false);

            let mut in_text = false;
            let mut current_para = String::new();
            let mut buf = Vec::new();

            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Start(ref e)) => {
                        if e.local_name().as_ref() == b"t" {
                            in_text = true;
                        }
                    }
                    Ok(Event::End(ref e)) => {
                        let local = e.local_name();
                        if local.as_ref() == b"t" {
                            in_text = false;
                        } else if local.as_ref() == b"p" {
                            let trimmed = current_para.trim();
                            if !trimmed.is_empty() {
                                output.push_str(trimmed);
                                output.push('\n');
                            }
                            current_para.clear();
                        }
                    }
                    Ok(Event::Text(ref e)) => {
                        if in_text {
                            if let Ok(txt) = e.unescape() {
                                current_para.push_str(&txt);
                            }
                        }
                    }
                    Ok(Event::Eof) => break,
                    _ => {}
                }
                buf.clear();
            }

            let rem = current_para.trim();
            if !rem.is_empty() {
                output.push_str(rem);
                output.push('\n');
            }
            output.push('\n');
        }

        Ok(output.trim().to_string())
    }

    /// Splits document text into structured chunks respecting paragraph boundaries.
    fn create_chunks(
        &self,
        file_path: &str,
        file_name: &str,
        text: &str,
        content_type: &str,
    ) -> Result<Vec<ChunkPayload>, String> {
        let paragraphs: Vec<&str> = text.split("\n\n").map(|p| p.trim()).filter(|p| !p.is_empty()).collect();
        if paragraphs.is_empty() {
            return Ok(Vec::new());
        }

        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_idx = 0;
        let mut start_line = 1;
        let mut current_line = 1;

        for para in paragraphs {
            let para_lines = para.lines().count() + 1;
            if !current_chunk.is_empty() && (current_chunk.len() + para.len() + 2 > self.max_chunk_chars) {
                let breadcrumb = format!("// Context: {} > Section {}", file_name, chunk_idx + 1);
                let full_text = format!("{}\n\n{}", breadcrumb, current_chunk.trim());
                chunks.push(ChunkPayload {
                    id: format!("{}_office_{}", file_name, chunk_idx),
                    text: full_text,
                    file_name: file_name.to_string(),
                    file_path: file_path.to_string(),
                    header_path: Some(format!("{} > Section {}", file_name, chunk_idx + 1)),
                    start_line,
                    end_line: current_line,
                    content_type: content_type.to_string(),
                    chunk_index: chunk_idx,
                });
                chunk_idx += 1;
                current_chunk.clear();
                start_line = current_line;
            }

            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(para);
            current_line += para_lines;
        }

        if !current_chunk.trim().is_empty() {
            let breadcrumb = format!("// Context: {} > Section {}", file_name, chunk_idx + 1);
            let full_text = format!("{}\n\n{}", breadcrumb, current_chunk.trim());
            chunks.push(ChunkPayload {
                id: format!("{}_office_{}", file_name, chunk_idx),
                text: full_text,
                file_name: file_name.to_string(),
                file_path: file_path.to_string(),
                header_path: Some(format!("{} > Section {}", file_name, chunk_idx + 1)),
                start_line,
                end_line: current_line,
                content_type: content_type.to_string(),
                chunk_index: chunk_idx,
            });
        }

        Ok(chunks)
    }
}

impl Chunker for OfficeChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let p = Path::new(file_path);
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("document").to_string();
        self.create_chunks(file_path, &file_name, content, "Office Document")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_office_chunker_supports_extension() {
        let chunker = OfficeChunker::default();
        assert!(chunker.supports_extension("docx"));
        assert!(chunker.supports_extension("pptx"));
        assert!(chunker.supports_extension("DOCX"));
        assert!(!chunker.supports_extension("pdf"));
        assert!(!chunker.supports_extension("xlsx"));
    }

    #[test]
    fn test_create_chunks_paragraph_flow() {
        let chunker = OfficeChunker::new(200, 20);
        let text = "Paragraph 1 with critical architectural context.\n\nParagraph 2 with deep details.\n\nParagraph 3 with conclusion.";
        let chunks = chunker.create_chunks("doc.docx", "doc.docx", text, "Word Document").unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks[0].text.contains("// Context: doc.docx > Section 1"));
        assert_eq!(chunks[0].content_type, "Word Document");
    }
}
