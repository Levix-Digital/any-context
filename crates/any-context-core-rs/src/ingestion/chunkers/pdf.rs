use std::collections::BTreeMap;
use std::path::Path;
use lopdf::{Document, Encoding, Object, ObjectId};
use crate::ingestion::chunkers::code::splitter;
use crate::models::ChunkPayload;

/// Represents a single text span extracted with 2D spatial coordinates.
#[derive(Debug, Clone)]
pub struct TextSpan {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub font_size: f32,
    pub text: String,
}

#[derive(Debug, Clone)]
struct TextCell {
    x: f32,
    width: f32,
    text: String,
    font_size: f32,
}

#[derive(Debug, Clone)]
struct TextLine {
    y: f32,
    cells: Vec<TextCell>,
}

/// Multiplies 3x3 affine 2D transformation matrices represented as [a, b, c, d, e, f]:
/// [ a  b  0 ]
/// [ c  d  0 ]
/// [ e  f  1 ]
#[inline]
fn multiply_matrix(m1: &[f32; 6], m2: &[f32; 6]) -> [f32; 6] {
    [
        m1[0] * m2[0] + m1[1] * m2[2],
        m1[0] * m2[1] + m1[1] * m2[3],
        m1[2] * m2[0] + m1[3] * m2[2],
        m1[2] * m2[1] + m1[3] * m2[3],
        m1[4] * m2[0] + m1[5] * m2[2] + m2[4],
        m1[4] * m2[1] + m1[5] * m2[3] + m2[5],
    ]
}

#[inline]
fn transform_point(m: &[f32; 6], x: f32, y: f32) -> (f32, f32) {
    (x * m[0] + y * m[2] + m[4], x * m[1] + y * m[3] + m[5])
}

/// Extracts text spans with accurate (X, Y) page coordinates from decoded content operations.
pub fn extract_spans_from_operations(
    operations: &[lopdf::content::Operation],
    encodings: &BTreeMap<Vec<u8>, Encoding>,
    initial_ctm: [f32; 6],
) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    let mut ctm = initial_ctm;
    let mut ctm_stack: Vec<[f32; 6]> = Vec::new();
    let mut tm = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut tlm = [1.0f32, 0.0, 0.0, 1.0, 0.0, 0.0];
    let mut font_size = 10.0f32;
    let mut leading = 12.0f32;
    let mut current_encoding: Option<&Encoding> = None;

    for op in operations {
        match op.operator.as_str() {
            "q" => ctm_stack.push(ctm),
            "Q" => {
                if let Some(saved) = ctm_stack.pop() {
                    ctm = saved;
                }
            }
            "cm" => {
                if op.operands.len() >= 6 {
                    let m = [
                        op.operands[0].as_float().unwrap_or(1.0),
                        op.operands[1].as_float().unwrap_or(0.0),
                        op.operands[2].as_float().unwrap_or(0.0),
                        op.operands[3].as_float().unwrap_or(1.0),
                        op.operands[4].as_float().unwrap_or(0.0),
                        op.operands[5].as_float().unwrap_or(0.0),
                    ];
                    ctm = multiply_matrix(&m, &ctm);
                }
            }
            "BT" => {
                tm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                tlm = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
            }
            "ET" => {
                // End text object
            }
            "Tf" => {
                if let Some(name) = op.operands.first().and_then(|o| o.as_name().ok()) {
                    current_encoding = encodings.get(name);
                }
                if let Some(sz) = op.operands.get(1).and_then(|o| o.as_float().ok()) {
                    font_size = sz;
                    leading = sz * 1.2;
                }
            }
            "TL" => {
                if let Some(ld) = op.operands.first().and_then(|o| o.as_float().ok()) {
                    leading = ld;
                }
            }
            "Tm" => {
                if op.operands.len() >= 6 {
                    let m = [
                        op.operands[0].as_float().unwrap_or(1.0),
                        op.operands[1].as_float().unwrap_or(0.0),
                        op.operands[2].as_float().unwrap_or(0.0),
                        op.operands[3].as_float().unwrap_or(1.0),
                        op.operands[4].as_float().unwrap_or(0.0),
                        op.operands[5].as_float().unwrap_or(0.0),
                    ];
                    tm = m;
                    tlm = m;
                }
            }
            "Td" => {
                if op.operands.len() >= 2 {
                    let tx = op.operands[0].as_float().unwrap_or(0.0);
                    let ty = op.operands[1].as_float().unwrap_or(0.0);
                    let move_m = [1.0, 0.0, 0.0, 1.0, tx, ty];
                    tlm = multiply_matrix(&move_m, &tlm);
                    tm = tlm;
                }
            }
            "TD" => {
                if op.operands.len() >= 2 {
                    let tx = op.operands[0].as_float().unwrap_or(0.0);
                    let ty = op.operands[1].as_float().unwrap_or(0.0);
                    leading = -ty;
                    let move_m = [1.0, 0.0, 0.0, 1.0, tx, ty];
                    tlm = multiply_matrix(&move_m, &tlm);
                    tm = tlm;
                }
            }
            "T*" => {
                let move_m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                tlm = multiply_matrix(&move_m, &tlm);
                tm = tlm;
            }
            "'" => {
                let move_m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                tlm = multiply_matrix(&move_m, &tlm);
                tm = tlm;

                let mut text = String::new();
                if let Some(enc) = current_encoding {
                    for operand in &op.operands {
                        if let Object::String(bytes, _) = operand {
                            let _ = enc.write_to_string(bytes, &mut text);
                        }
                    }
                }
                let clean = text.trim();
                if !clean.is_empty() {
                    let (px, py) = transform_point(&ctm, tm[4], tm[5]);
                    let est_width = clean.chars().count() as f32 * font_size * 0.42;
                    spans.push(TextSpan {
                        x: px,
                        y: py,
                        width: est_width,
                        font_size,
                        text: clean.to_string(),
                    });
                }
            }
            "\"" => {
                let move_m = [1.0, 0.0, 0.0, 1.0, 0.0, -leading];
                tlm = multiply_matrix(&move_m, &tlm);
                tm = tlm;

                let mut text = String::new();
                if let Some(enc) = current_encoding {
                    if let Some(Object::String(bytes, _)) = op.operands.get(2) {
                        let _ = enc.write_to_string(bytes, &mut text);
                    }
                }
                let clean = text.trim();
                if !clean.is_empty() {
                    let (px, py) = transform_point(&ctm, tm[4], tm[5]);
                    let est_width = clean.chars().count() as f32 * font_size * 0.42;
                    spans.push(TextSpan {
                        x: px,
                        y: py,
                        width: est_width,
                        font_size,
                        text: clean.to_string(),
                    });
                }
            }
            "Tj" | "TJ" => {
                let mut text = String::new();
                if let Some(enc) = current_encoding {
                    for operand in &op.operands {
                        match operand {
                            Object::String(bytes, _) => {
                                let _ = enc.write_to_string(bytes, &mut text);
                            }
                            Object::Array(arr) => {
                                for item in arr {
                                    if let Object::String(bytes, _) = item {
                                        let _ = enc.write_to_string(bytes, &mut text);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                let clean = text.trim();
                if !clean.is_empty() {
                    let (px, py) = transform_point(&ctm, tm[4], tm[5]);
                    let est_width = clean.chars().count() as f32 * font_size * 0.42;
                    spans.push(TextSpan {
                        x: px,
                        y: py,
                        width: est_width,
                        font_size,
                        text: clean.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    spans
}

/// Extracts all text spans from a PDF page, including nested Form XObjects.
pub fn extract_page_spans(doc: &Document, page_id: ObjectId) -> Vec<TextSpan> {
    let encodings: BTreeMap<Vec<u8>, Encoding> = doc
        .get_page_fonts(page_id)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(name, font)| {
            font.get_font_encoding(doc).ok().map(|enc| (name, enc))
        })
        .collect();

    let content_bytes = doc.get_page_content(page_id);
    let mut spans = if let Ok(content) = lopdf::content::Content::decode(&content_bytes) {
        extract_spans_from_operations(&content.operations, &encodings, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    } else {
        Vec::new()
    };

    // Also inspect Form XObjects referenced in page resources
    if let Ok((Some(resources), _)) = doc.get_page_resources(page_id) {
        if let Ok(xobjects) = resources.get(b"XObject").and_then(Object::as_dict) {
            for (_, xobj_ref) in xobjects.iter() {
                if let Ok(obj_id) = xobj_ref.as_reference() {
                    if let Ok(stream) = doc.get_object(obj_id).and_then(Object::as_stream) {
                        let is_form = stream.dict.get(b"Subtype")
                            .and_then(Object::as_name)
                            .map(|s| s.eq_ignore_ascii_case(b"Form"))
                            .unwrap_or(false);

                        if is_form {
                            if let Ok(form_content) = stream.decode_content() {
                                let form_spans = extract_spans_from_operations(
                                    &form_content.operations,
                                    &encodings,
                                    [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                                );
                                spans.extend(form_spans);
                            }
                        }
                    }
                }
            }
        }
    }

    spans
}

/// Universal, domain-agnostic 2D spatial layout reconstruction.
/// Groups text spans by vertical coordinate Y (line banding) and horizontal gaps X,
/// reconstructing tables, forms, and paragraphs into readable Markdown.
pub fn reconstruct_spatial_layout(spans: &mut [TextSpan]) -> String {
    if spans.is_empty() {
        return String::new();
    }

    // 1. Calculate dominant body font size (weighted by character count)
    let mut font_counts: BTreeMap<u32, usize> = BTreeMap::new();
    for s in spans.iter() {
        let key = (s.font_size * 10.0).round() as u32;
        *font_counts.entry(key).or_insert(0) += s.text.chars().count();
    }
    let body_font_size = font_counts
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(&k, _)| k as f32 / 10.0)
        .unwrap_or(10.0)
        .max(6.0);

    // 2. Sort spans by Y descending (top to bottom), then X ascending (left to right)
    spans.sort_by(|a, b| {
        b.y.partial_cmp(&a.y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal))
    });

    // 3. Line banding with dynamic vertical tolerance
    let mut lines: Vec<TextLine> = Vec::new();
    for span in spans.iter() {
        let tolerance = (span.font_size * 0.45).max(2.5);
        let matched_line = lines.iter_mut().find(|l| (l.y - span.y).abs() <= tolerance);

        match matched_line {
            Some(line) => {
                if let Some(last_cell) = line.cells.last_mut() {
                    let gap = span.x - (last_cell.x + last_cell.width);
                    if gap <= span.font_size * 1.5 && gap >= -span.font_size * 0.5 {
                        // Merge into existing cell
                        if gap > span.font_size * 0.25 {
                            last_cell.text.push(' ');
                        }
                        last_cell.text.push_str(&span.text);
                        last_cell.width = (span.x + span.width) - last_cell.x;
                    } else {
                        // New distinct column / cell
                        line.cells.push(TextCell {
                            x: span.x,
                            width: span.width,
                            text: span.text.clone(),
                            font_size: span.font_size,
                        });
                    }
                } else {
                    line.cells.push(TextCell {
                        x: span.x,
                        width: span.width,
                        text: span.text.clone(),
                        font_size: span.font_size,
                    });
                }
            }
            None => {
                lines.push(TextLine {
                    y: span.y,
                    cells: vec![TextCell {
                        x: span.x,
                        width: span.width,
                        text: span.text.clone(),
                        font_size: span.font_size,
                    }],
                });
            }
        }
    }

    // 4. Sort lines by Y descending, and cells within lines by X ascending
    lines.sort_by(|a, b| b.y.partial_cmp(&a.y).unwrap_or(std::cmp::Ordering::Equal));
    for line in &mut lines {
        line.cells.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal));
    }

    // 5. Project into structured Markdown
    let mut markdown = String::new();
    let mut in_table = false;
    let mut table_cols = 0;

    for line in &lines {
        let cell_count = line.cells.len();
        if cell_count > 1 {
            // Multi-column line -> Format as Markdown table
            if !in_table || table_cols != cell_count {
                if in_table {
                    markdown.push('\n');
                }
                let header_row: Vec<String> = line.cells.iter().map(|c| c.text.replace('|', "\\|")).collect();
                markdown.push_str(&format!("| {} |\n", header_row.join(" | ")));
                let sep_row: Vec<&str> = (0..cell_count).map(|_| "---").collect();
                markdown.push_str(&format!("| {} |\n", sep_row.join(" | ")));
                in_table = true;
                table_cols = cell_count;
            } else {
                let data_row: Vec<String> = line.cells.iter().map(|c| c.text.replace('|', "\\|")).collect();
                markdown.push_str(&format!("| {} |\n", data_row.join(" | ")));
            }
        } else if let Some(cell) = line.cells.first() {
            // Single-cell line -> Heading or paragraph
            if in_table {
                markdown.push('\n');
                in_table = false;
                table_cols = 0;
            }

            if cell.font_size >= body_font_size * 1.8 {
                markdown.push_str(&format!("# {}\n\n", cell.text));
            } else if cell.font_size >= body_font_size * 1.35 {
                markdown.push_str(&format!("## {}\n\n", cell.text));
            } else {
                markdown.push_str(&cell.text);
                markdown.push('\n');
            }
        }
    }

    if in_table {
        markdown.push('\n');
    }

    markdown.trim().to_string()
}

/// High-performance native PDF Layout and Text Chunker.
/// Implements Nível 1 of the Smart Cascade with 2D Spatial Reconstruction:
/// - Decomposes PDF files by page.
/// - Extracts text streams and reconstructs 2D layout (columns, tables, headers).
/// - Quality Gate: Evaluates text density per page.
/// - If page text >= min_text_chars: generates structured markdown with breadcrumbs:
///   `// Context: <file>.pdf > Page <N>`
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

    /// Chunks raw byte content of a PDF file using 2D spatial layout reconstruction.
    pub fn chunk_bytes(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf")
            .to_string();

        let doc = match Document::load_mem(bytes) {
            Ok(d) => d,
            Err(e) => {
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

        for (&page_num, &page_id) in &pages {
            // Reconstruct 2D spatial layout from text spans
            let mut spans = extract_page_spans(&doc, page_id);
            let layout_text = if !spans.is_empty() {
                reconstruct_spatial_layout(&mut spans)
            } else {
                // Fallback to naive extract_text if no spans detected
                doc.extract_text(&[page_num]).unwrap_or_default()
            };

            let clean_text = layout_text.trim();

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

    #[test]
    fn test_spatial_layout_reconstruction_synthetic() {
        let mut spans = vec![
            TextSpan { x: 40.0, y: 700.0, width: 60.0, font_size: 10.0, text: "Col A".to_string() },
            TextSpan { x: 200.0, y: 700.0, width: 60.0, font_size: 10.0, text: "Col B".to_string() },
            TextSpan { x: 40.0, y: 680.0, width: 50.0, font_size: 10.0, text: "Val 1".to_string() },
            TextSpan { x: 200.0, y: 680.0, width: 50.0, font_size: 10.0, text: "Val 2".to_string() },
        ];

        let md = reconstruct_spatial_layout(&mut spans);
        assert!(md.contains("| Col A | Col B |"));
        assert!(md.contains("| --- | --- |"));
        assert!(md.contains("| Val 1 | Val 2 |"));
    }

    #[test]
    fn test_spatial_layout_heading_detection() {
        let mut spans = vec![
            TextSpan { x: 40.0, y: 800.0, width: 150.0, font_size: 20.0, text: "Big Document Title".to_string() },
            TextSpan { x: 40.0, y: 750.0, width: 100.0, font_size: 14.0, text: "Section Header".to_string() },
            TextSpan { x: 40.0, y: 700.0, width: 200.0, font_size: 10.0, text: "This is body paragraph line 1.".to_string() },
            TextSpan { x: 40.0, y: 685.0, width: 200.0, font_size: 10.0, text: "This is body paragraph line 2.".to_string() },
        ];

        let md = reconstruct_spatial_layout(&mut spans);
        assert!(md.contains("# Big Document Title"));
        assert!(md.contains("## Section Header"));
        assert!(md.contains("This is body paragraph line 1."));
    }

    #[test]
    fn test_cmr_real_document_separation() {
        let path = r#"C:\Users\guilh\OneDrive\Documents\Documentos\Outros\Shipment Checklist\2026\01\01\CMR for Single Pickup Report.pdf"#;
        if let Ok(bytes) = std::fs::read(path) {
            let chunker = PdfChunker::new(4000);
            let chunks = chunker.chunk_bytes(path, &bytes).unwrap();
            assert!(!chunks.is_empty());
            let text = &chunks[0].text;

            // In the naive parser, these were merged as "IKEA CALGARYBISON TRANSPORT INC."
            // In our 2D spatial layout parser, they must be separated!
            assert!(!text.contains("IKEA CALGARYBISON TRANSPORT INC."));
            assert!(text.contains("IKEA CALGARY"));
            assert!(text.contains("BISON TRANSPORT INC."));
            // They should be in a table row together separated by pipe:
            assert!(text.contains("| IKEA CALGARY | BISON TRANSPORT INC. |"));
        }
    }
}
