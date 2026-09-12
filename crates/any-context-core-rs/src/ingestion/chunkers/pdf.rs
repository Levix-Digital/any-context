use std::path::Path;
use lopdf::Document;
use crate::ingestion::chunkers::code::splitter;
use crate::models::ChunkPayload;

/// High-performance native PDF Layout and Text Chunker.
/// Implements Nível 1 of the Smart Cascade:
/// - Decomposes PDF files by page.
/// - Extracts text streams and layout blocks.
/// - Quality Gate: Evaluates text density per page.
/// - If page text >= min_text_chars: generates structured markdown with breadcrumbs:
///   `// Context: <file>.pdf > Page <N> > rows X..Y`
/// - If page text < min_text_chars (scanned page / image-only): flags as Scanned Page
///   and emits context header for Nível 2 (OCR Gate).
#[derive(Debug, Clone)]
pub struct PdfChunker {
    pub max_chunk_chars: usize,
    pub min_text_chars: usize,
}

impl Default for PdfChunker {
    fn default() -> Self {
        Self::new(1800)
    }
}

impl PdfChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            max_chunk_chars,
            min_text_chars: 50,
        }
    }

    pub fn supports_extension(&self, ext: &str) -> bool {
        ext.eq_ignore_ascii_case("pdf")
    }

    /// Reads and chunks a PDF file directly from the filesystem in native Rust.
    pub fn chunk_file(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        let bytes = std::fs::read(file_path).map_err(|e| {
            format!("Failed to read PDF file '{}': {}", file_path, e)
        })?;
        self.chunk_bytes(file_path, &bytes)
    }

    /// Chunks raw byte content of a PDF file.
    pub fn chunk_bytes(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf")
            .to_string();

        let doc = match Document::load_mem(bytes) {
            Ok(d) => d,
            Err(e) => {
                // Graceful fallback for corrupted / non-standard PDF
                let fallback_text = format!(
                    "// Context: {} > [Unparsed PDF Document]\n\n[Warning: PDF could not be parsed: {} | Size: {} bytes]",
                    file_name, e, bytes.len()
                );
                return Ok(vec![ChunkPayload {
                    id: format!("{}_pdf_0", file_name),
                    text: fallback_text,
                    file_name: file_name.clone(),
                    file_path: file_path.to_string(),
                    header_path: Some(format!("{} > [Unparsed PDF]", file_name)),
                    start_line: 1,
                    end_line: 1,
                    content_type: "pdf".to_string(),
                    chunk_index: 0,
                }]);
            }
        };

        let pages = doc.get_pages();
        let total_pages = pages.len();
        let mut chunks = Vec::new();
        let mut chunk_index = 0;

        if total_pages == 0 {
            let empty_text = format!("// Context: {} > [Empty Document]\n\n[Empty PDF Document]", file_name);
            chunks.push(ChunkPayload {
                id: format!("{}_pdf_0", file_name),
                text: empty_text,
                file_name: file_name.clone(),
                file_path: file_path.to_string(),
                header_path: Some(format!("{} > [Empty Document]", file_name)),
                start_line: 1,
                end_line: 1,
                content_type: "pdf".to_string(),
                chunk_index: 0,
            });
            return Ok(chunks);
        }

        for (&page_num, &_page_id) in &pages {
            // Extract text from the page
            let extracted = doc.extract_text(&[page_num]).unwrap_or_default();
            let clean_text = extracted.trim();

            // Quality Gate: Does page have sufficient text density?
            if clean_text.len() >= self.min_text_chars {
                // Digital Page (Nível 1)
                let header = format!("{} > Page {}", file_name, page_num);
                let breadcrumb = format!("// Context: {}\n\n", header);

                if clean_text.len() <= self.max_chunk_chars {
                    let full_chunk_text = format!("{}{}", breadcrumb, clean_text);
                    let line_count = full_chunk_text.lines().count().max(1);
                    chunks.push(ChunkPayload {
                        id: format!("{}_pdf_{}", file_name, chunk_index),
                        text: full_chunk_text,
                        file_name: file_name.clone(),
                        file_path: file_path.to_string(),
                        header_path: Some(header),
                        start_line: 1,
                        end_line: line_count,
                        content_type: "pdf".to_string(),
                        chunk_index,
                    });
                    chunk_index += 1;
                } else {
                    // Split oversized page content cleanly
                    let slices = splitter::split_oversized_code(clean_text, self.max_chunk_chars);
                    for (slice_idx, slice) in slices.into_iter().enumerate() {
                        let slice_header = format!("{} > Page {} (part {})", file_name, page_num, slice_idx + 1);
                        let full_chunk_text = format!("// Context: {}\n\n{}", slice_header, slice.text);
                        let line_count = full_chunk_text.lines().count().max(1);
                        chunks.push(ChunkPayload {
                            id: format!("{}_pdf_{}", file_name, chunk_index),
                            text: full_chunk_text,
                            file_name: file_name.clone(),
                            file_path: file_path.to_string(),
                            header_path: Some(slice_header),
                            start_line: 1,
                            end_line: line_count,
                            content_type: "pdf".to_string(),
                            chunk_index,
                        });
                        chunk_index += 1;
                    }
                }
            } else {
                // Scanned Page / Image-Only Page (Nível 2 Gate)
                let header = format!("{} > Page {} (Scanned Page)", file_name, page_num);
                let scan_notice = if clean_text.is_empty() {
                    format!(
                        "// Context: {}\n\n[Scanned Page: Page {} of {} contains image-based or rasterized content without embedded text layer.]",
                        header, page_num, total_pages
                    )
                } else {
                    format!(
                        "// Context: {}\n\n[Scanned Page: Page {} of {} contains low-density text ({} chars):\n{}]",
                        header, page_num, total_pages, clean_text.len(), clean_text
                    )
                };

                let line_count = scan_notice.lines().count().max(1);
                chunks.push(ChunkPayload {
                    id: format!("{}_pdf_{}", file_name, chunk_index),
                    text: scan_notice,
                    file_name: file_name.clone(),
                    file_path: file_path.to_string(),
                    header_path: Some(header),
                    start_line: 1,
                    end_line: line_count,
                    content_type: "pdf_scan".to_string(),
                    chunk_index,
                });
                chunk_index += 1;
            }
        }

        Ok(chunks)
    }

    /// Chunks pre-extracted text representation of a PDF.
    pub fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf")
            .to_string();

        let clean_content = content.trim();
        if clean_content.is_empty() {
            return Ok(Vec::new());
        }

        let header = format!("{} > Content", file_name);
        let breadcrumb = format!("// Context: {}\n\n", header);

        if clean_content.len() <= self.max_chunk_chars {
            let full_text = format!("{}{}", breadcrumb, clean_content);
            let line_count = full_text.lines().count().max(1);
            return Ok(vec![ChunkPayload {
                id: format!("{}_pdf_0", file_name),
                text: full_text,
                file_name: file_name.clone(),
                file_path: file_path.to_string(),
                header_path: Some(header),
                start_line: 1,
                end_line: line_count,
                content_type: "pdf".to_string(),
                chunk_index: 0,
            }]);
        }

        let slices = splitter::split_oversized_code(clean_content, self.max_chunk_chars);
        let mut chunks = Vec::new();
        for (idx, slice) in slices.into_iter().enumerate() {
            let slice_header = format!("{} > Section {}", file_name, idx + 1);
            let full_text = format!("// Context: {}\n\n{}", slice_header, slice.text);
            let line_count = full_text.lines().count().max(1);
            chunks.push(ChunkPayload {
                id: format!("{}_pdf_{}", file_name, idx),
                text: full_text,
                file_name: file_name.clone(),
                file_path: file_path.to_string(),
                header_path: Some(slice_header),
                start_line: 1,
                end_line: line_count,
                content_type: "pdf".to_string(),
                chunk_index: idx,
            });
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdf_chunk_text_fallback() {
        let chunker = PdfChunker::new(500);
        let chunks = chunker.chunk("sample.pdf", "Hello PDF world this is page 1.").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "pdf");
        assert!(chunks[0].header_path.as_ref().unwrap().contains("sample.pdf"));
    }

    #[test]
    fn test_supports_pdf_extension() {
        let chunker = PdfChunker::new(500);
        assert!(chunker.supports_extension("pdf"));
        assert!(chunker.supports_extension("PDF"));
        assert!(!chunker.supports_extension("txt"));
    }
}
