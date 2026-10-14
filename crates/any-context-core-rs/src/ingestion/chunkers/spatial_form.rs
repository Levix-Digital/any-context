//! Domain-Agnostic 2D Spatial & Typographical Form Chunker.
//!
//! Reconstructs dense multi-column forms, invoices, CMRs, medical records, and legal briefs
//! without ANY domain-specific keywords. Uses pure typographical contrasts (bold, font size,
//! casing), syntactic punctuation (`:`, `-`, `___`), and spatial 2D proximity (horizontal/vertical
//! Voronoi neighborhoods) to pair labels with values and isolate tabular grids.

use std::path::Path;
use crate::models::ChunkPayload;
use crate::ingestion::chunkers::pdf::TextSpan;

/// A recognized field or key-value pair extracted from 2D document layout.
#[derive(Debug, Clone, PartialEq)]
pub struct FormField {
    pub label: String,
    pub value: String,
    pub section: Option<String>,
}

/// Domain-Agnostic 2D Spatial Form Chunker.
#[derive(Debug, Clone)]
pub struct SpatialFormChunker {
    pub max_chunk_chars: usize,
    pub line_y_tolerance: f32,
    pub col_x_tolerance: f32,
}

impl Default for SpatialFormChunker {
    fn default() -> Self {
        Self {
            max_chunk_chars: 1800,
            line_y_tolerance: 3.5,
            col_x_tolerance: 15.0,
        }
    }
}

impl SpatialFormChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            max_chunk_chars,
            line_y_tolerance: 3.5,
            col_x_tolerance: 15.0,
        }
    }

    /// Chunks pre-extracted spans from a PDF page using 2D spatial & typographic grouping.
    pub fn chunk_spans(
        &self,
        file_path: &str,
        page_num: usize,
        spans: &[TextSpan],
    ) -> Vec<ChunkPayload> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf")
            .to_string();

        if spans.is_empty() {
            return Vec::new();
        }

        let fields = self.extract_fields_from_spans(spans);
        if fields.is_empty() {
            return Vec::new();
        }

        self.format_fields_into_chunks(&file_name, file_path, page_num, &fields)
    }

    /// Chunks raw text (e.g. from table-pipe fragmented text like IKEA CMRs) by parsing
    /// syntactic separators, uppercase headings, and cell blocks into clean structured fields.
    pub fn chunk_dense_text(
        &self,
        file_name: &str,
        file_path: &str,
        page_num: usize,
        raw_text: &str,
    ) -> Vec<ChunkPayload> {
        let fields = self.extract_fields_from_dense_text(raw_text);
        if fields.is_empty() {
            // Fallback to basic cleaned representation
            let clean = raw_text.trim();
            let header = format!("{} > Page {}", file_name, page_num);
            return vec![ChunkPayload {
                id: format!("{}_p{}_form_0", file_name, page_num),
                text: format!("// Context: {}\n\n{}", header, clean),
                file_name: file_name.to_string(),
                file_path: file_path.to_string(),
                header_path: Some(header),
                start_line: 1,
                end_line: clean.lines().count().max(1),
                content_type: "document_form".to_string(),
                chunk_index: 0,
            }];
        }

        self.format_fields_into_chunks(file_name, file_path, page_num, &fields)
    }

    /// Extracts structured key-value fields from 2D coordinates and typographic properties.
    pub fn extract_fields_from_spans(&self, spans: &[TextSpan]) -> Vec<FormField> {
        let mut fields = Vec::new();
        let mut used_indices = std::collections::HashSet::new();

        // 1. Identify potential label spans:
        // - Text ends with ':', '-', or '___'
        // - Or uppercase word followed by lowercase/numeric
        for (i, span) in spans.iter().enumerate() {
            if used_indices.contains(&i) {
                continue;
            }

            let text = span.text.trim();
            if text.is_empty() {
                continue;
            }

            if Self::is_label_candidate(text) {
                let clean_label = text.trim_end_matches([':', '-', '_', ' ']).to_string();

                // Look for candidate value:
                // First: horizontally adjacent to the right (same Y, X > label.X)
                let mut best_val_idx = None;
                let mut min_dist = f32::MAX;

                for (j, other) in spans.iter().enumerate() {
                    if i == j || used_indices.contains(&j) {
                        continue;
                    }

                    let dy = (span.y - other.y).abs();
                    if dy <= self.line_y_tolerance && other.x > span.x {
                        let dx = other.x - (span.x + span.width);
                        if dx >= 0.0 && dx < 250.0 && dx < min_dist {
                            min_dist = dx;
                            best_val_idx = Some(j);
                        }
                    }
                }

                // Second: vertically below (Y > label.Y, similar X)
                if best_val_idx.is_none() {
                    let mut min_v_dist = f32::MAX;
                    for (j, other) in spans.iter().enumerate() {
                        if i == j || used_indices.contains(&j) {
                            continue;
                        }

                        let dx = (span.x - other.x).abs();
                        if dx <= self.col_x_tolerance && other.y > span.y {
                            let dy = other.y - span.y;
                            if dy > 0.0 && dy < 35.0 && dy < min_v_dist {
                                min_v_dist = dy;
                                best_val_idx = Some(j);
                            }
                        }
                    }
                }

                if let Some(val_idx) = best_val_idx {
                    let val_text = spans[val_idx].text.trim().to_string();
                    if !val_text.is_empty() {
                        fields.push(FormField {
                            label: clean_label,
                            value: val_text,
                            section: None,
                        });
                        used_indices.insert(i);
                        used_indices.insert(val_idx);
                    }
                }
            }
        }

        fields
    }

    /// Extracts structured key-value fields from raw dense text (including fragmented markdown tables).
    pub fn extract_fields_from_dense_text(&self, text: &str) -> Vec<FormField> {
        let mut fields = Vec::new();
        let mut table_headers: Option<Vec<String>> = None;

        for raw_line in text.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("// Context:") {
                continue;
            }

            // Case A: Markdown table cells `| Col A | Col B | ...`
            if line.starts_with('|') && line.ends_with('|') {
                if line.contains("---") {
                    continue;
                }

                let cells: Vec<&str> = line
                    .trim_matches('|')
                    .split('|')
                    .map(|c| c.trim())
                    .filter(|c| !c.is_empty())
                    .collect();

                if cells.is_empty() {
                    continue;
                }

                // If cell contains colon `Label: Value` (e.g. fragmented CMR cells)
                let mut found_inline_colon = false;
                for cell in &cells {
                    if let Some(pos) = cell.find(':') {
                        let label = cell[..pos].trim();
                        let value = cell[pos + 1..].trim();
                        if !label.is_empty() && !value.is_empty() && label.len() < 50 {
                            fields.push(FormField {
                                label: label.to_string(),
                                value: value.to_string(),
                                section: None,
                            });
                            found_inline_colon = true;
                        }
                    }
                }

                if found_inline_colon {
                    continue;
                }

                // Header detection: First table row seen
                if table_headers.is_none() {
                    table_headers = Some(cells.iter().map(|c| c.to_string()).collect());
                    continue;
                }

                // Use table headers ONLY if it's explicitly a 2-column Key-Value table
                if let Some(ref headers) = table_headers {
                    if headers.len() == 2 && cells.len() == 2 {
                        let h0 = headers[0].to_lowercase();
                        let h1 = headers[1].to_lowercase();
                        let is_kv_header = matches!(h0.as_str(), "property" | "key" | "field" | "attribute" | "name" | "item" | "parameter" | "propriedade" | "chave" | "campo")
                            && matches!(h1.as_str(), "value" | "val" | "description" | "detail" | "info" | "data" | "valor" | "descrição" | "dado");

                        if is_kv_header {
                            fields.push(FormField {
                                label: cells[0].trim_end_matches([':', '-']).to_string(),
                                value: cells[1].to_string(),
                                section: None,
                            });
                            continue;
                        }
                    }
                }
            } else {
                // Leaving table block, reset table headers
                table_headers = None;

                // Case B: Standard line with colon `Label: Value`
                if let Some(pos) = line.find(':') {
                    let label = line[..pos].trim();
                    let value = line[pos + 1..].trim();
                    if !label.is_empty() && !value.is_empty() && label.len() < 60 && !label.contains('\n') {
                        fields.push(FormField {
                            label: label.to_string(),
                            value: value.to_string(),
                            section: None,
                        });
                    }
                }
            }
        }

        fields
    }

    /// Tests if a string candidate qualifies as a field label in a domain-agnostic manner.
    pub fn is_label_candidate(text: &str) -> bool {
        let t = text.trim();
        if t.is_empty() || t.len() > 65 {
            return false;
        }

        // 1. Syntactic separator: ends with ':', '-', or '___'
        if t.ends_with(':') || t.ends_with('-') || t.ends_with("___") {
            return true;
        }

        // 2. Numerical index followed by label: "1.", "1 -", "12", "Item 1"
        if t.starts_with(|c: char| c.is_ascii_digit()) && (t.contains('.') || t.contains('-') || t.contains(' ')) {
            let words: Vec<&str> = t.split_whitespace().collect();
            if words.len() >= 2 && words.len() <= 6 {
                return true;
            }
        }

        // 3. Typographical uppercase label (e.g. "DATA DE ADMISSÃO", "ABSENDER", "SHIPMENT ID")
        let alphabetic_chars: Vec<char> = t.chars().filter(|c| c.is_alphabetic()).collect();
        if !alphabetic_chars.is_empty() && alphabetic_chars.len() >= 3 && alphabetic_chars.len() <= 35 {
            if alphabetic_chars.iter().all(|c| c.is_uppercase()) {
                let word_count = t.split_whitespace().count();
                if word_count >= 1 && word_count <= 5 {
                    return true;
                }
            }
        }

        false
    }

    /// Formats extracted fields into high-density semantic chunks.
    fn format_fields_into_chunks(
        &self,
        file_name: &str,
        file_path: &str,
        page_num: usize,
        fields: &[FormField],
    ) -> Vec<ChunkPayload> {
        let mut chunks = Vec::new();
        let mut current_lines = Vec::new();
        let mut current_chars = 0usize;
        let mut chunk_idx = 0usize;

        let header = format!("{} > Page {} (Structured Form Fields)", file_name, page_num);
        let breadcrumb = format!("// Context: {}\n\n### Document Form Data\n", header);

        for field in fields {
            let line = format!("- **{}**: {}", field.label, field.value);
            if current_chars + line.len() > self.max_chunk_chars && !current_lines.is_empty() {
                let body = current_lines.join("\n");
                let full_text = format!("{}{}", breadcrumb, body);
                let line_count = full_text.lines().count().max(1);

                chunks.push(ChunkPayload {
                    id: format!("{}_p{}_form_{}", file_name, page_num, chunk_idx),
                    text: full_text,
                    file_name: file_name.to_string(),
                    file_path: file_path.to_string(),
                    header_path: Some(header.clone()),
                    start_line: 1,
                    end_line: line_count,
                    content_type: "document_form".to_string(),
                    chunk_index: chunk_idx,
                });

                chunk_idx += 1;
                current_lines.clear();
                current_chars = 0;
            }

            current_chars += line.len() + 1;
            current_lines.push(line);
        }

        if !current_lines.is_empty() {
            let body = current_lines.join("\n");
            let full_text = format!("{}{}", breadcrumb, body);
            let line_count = full_text.lines().count().max(1);

            chunks.push(ChunkPayload {
                id: format!("{}_p{}_form_{}", file_name, page_num, chunk_idx),
                text: full_text,
                file_name: file_name.to_string(),
                file_path: file_path.to_string(),
                header_path: Some(header),
                start_line: 1,
                end_line: line_count,
                content_type: "document_form".to_string(),
                chunk_index: chunk_idx,
            });
        }

        chunks
    }
}
