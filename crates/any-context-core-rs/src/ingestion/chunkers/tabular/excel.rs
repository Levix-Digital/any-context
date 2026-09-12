use std::path::Path;
use calamine::{open_workbook_auto, open_workbook_auto_from_rs, Reader, Data};
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct ExcelChunker {
    pub max_chunk_chars: usize,
}

impl ExcelChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self { max_chunk_chars }
    }

    fn format_cell(cell: &Data) -> String {
        match cell {
            Data::Int(v) => v.to_string(),
            Data::Float(v) => {
                if v.fract() == 0.0 && *v >= i64::MIN as f64 && *v <= i64::MAX as f64 {
                    format!("{}", *v as i64)
                } else {
                    let s = format!("{:.4}", v);
                    s.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            }
            Data::String(s) => s.trim().replace('|', "\\|").replace('\n', " "),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(dt) => format!("{}", dt),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
            Data::Error(e) => format!("#ERR:{:?}", e),
            Data::Empty => String::new(),
        }
    }

    pub fn chunk_file(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        let path = Path::new(file_path);
        let mut workbook = open_workbook_auto(path)
            .map_err(|e| format!("Failed to open workbook '{}': {}", file_path, e))?;

        self.process_workbook(&mut workbook, file_path)
    }

    pub fn chunk_bytes(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        let cursor = std::io::Cursor::new(bytes);
        let mut workbook = open_workbook_auto_from_rs(cursor)
            .map_err(|e| format!("Failed to open workbook bytes for '{}': {}", file_path, e))?;

        self.process_workbook(&mut workbook, file_path)
    }

    fn process_workbook<R: std::io::Read + std::io::Seek>(
        &self,
        workbook: &mut calamine::Sheets<R>,
        file_path: &str,
    ) -> Result<Vec<ChunkPayload>, String> {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path)
            .to_string();

        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("xlsx")
            .to_lowercase();

        let content_type = if ext == "ods" { "ods" } else { "excel" };

        let mut chunks = Vec::new();
        let sheet_names = workbook.sheet_names();

        for sheet_name in sheet_names {
            if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                if range.is_empty() {
                    continue;
                }

                // Collect all non-empty rows
                let mut valid_rows: Vec<Vec<String>> = Vec::new();
                for row in range.rows() {
                    let formatted_cells: Vec<String> = row.iter().map(Self::format_cell).collect();
                    // Skip if entirely empty
                    if formatted_cells.iter().all(|c| c.is_empty()) {
                        continue;
                    }
                    valid_rows.push(formatted_cells);
                }

                if valid_rows.is_empty() {
                    continue;
                }

                // Row 0 is the headers
                let headers: Vec<String> = valid_rows[0].clone();
                let header_summary = format!("[{}]", headers.join(", "));

                // Build Markdown table header
                let table_header_line = format!("| {} |", headers.join(" | "));
                let table_separator_line = format!("|{}|", headers.iter().map(|_| "---").collect::<Vec<&str>>().join("|"));
                let header_block = format!("{}\n{}", table_header_line, table_separator_line);

                let num_data_rows = valid_rows.len().saturating_sub(1);

                // Multi-chunk row-windowing with sheet & header propagation
                let mut current_rows: Vec<String> = Vec::new();
                let mut current_start_row = 1;
                let mut current_chars = header_block.len() + 120; // prefix overhead

                for (idx, row) in valid_rows.iter().skip(1).enumerate() {
                    let row_idx = idx + 1; // 1-indexed data row
                    let formatted_row = format!("| {} |", row.join(" | "));
                    let row_len = formatted_row.len() + 1;

                    if !current_rows.is_empty() && (current_chars + row_len > self.max_chunk_chars) {
                        let end_row = row_idx - 1;
                        let header_path = format!(
                            "{} > Sheet: \"{}\" > {} > rows {}..{}",
                            file_name, sheet_name, header_summary, current_start_row, end_row
                        );
                        let table_content = format!("{}\n{}", header_block, current_rows.join("\n"));
                        let text = format!("// Context: {}\n{}", header_path, table_content);

                        chunks.push(ChunkPayload::new(
                            format!("{}_{}_chunk_{}", file_name, sheet_name, chunks.len()),
                            text,
                            file_name.clone(),
                            file_path.to_string(),
                            Some(header_path),
                            current_start_row,
                            end_row,
                            content_type.to_string(),
                            chunks.len(),
                        ));

                        current_rows.clear();
                        current_start_row = row_idx;
                        current_chars = header_block.len() + 120;
                    }

                    current_rows.push(formatted_row);
                    current_chars += row_len;
                }

                if !current_rows.is_empty() {
                    let end_row = num_data_rows;
                    let header_path = format!(
                        "{} > Sheet: \"{}\" > {} > rows {}..{}",
                        file_name, sheet_name, header_summary, current_start_row, end_row
                    );
                    let table_content = format!("{}\n{}", header_block, current_rows.join("\n"));
                    let text = format!("// Context: {}\n{}", header_path, table_content);

                    chunks.push(ChunkPayload::new(
                        format!("{}_{}_chunk_{}", file_name, sheet_name, chunks.len()),
                        text,
                        file_name.clone(),
                        file_path.to_string(),
                        Some(header_path),
                        current_start_row,
                        end_row,
                        content_type.to_string(),
                        chunks.len(),
                    ));
                }
            }
        }

        Ok(chunks)
    }
}
