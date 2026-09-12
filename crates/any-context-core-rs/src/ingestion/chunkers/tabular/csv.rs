use std::path::Path;
use crate::ingestion::chunkers::code::splitter;
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct CsvChunker {
    pub max_chunk_chars: usize,
}

impl CsvChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self { max_chunk_chars }
    }

    /// Auto-detects delimiter from the first few lines of content.
    fn detect_delimiter(content: &str, ext: &str) -> u8 {
        if ext == "tsv" {
            return b'\t';
        }
        let sample: String = content.lines().take(5).collect::<Vec<&str>>().join("\n");
        let comma_count = sample.chars().filter(|&c| c == ',').count();
        let semicolon_count = sample.chars().filter(|&c| c == ';').count();
        let tab_count = sample.chars().filter(|&c| c == '\t').count();

        if tab_count > comma_count && tab_count > semicolon_count {
            b'\t'
        } else if semicolon_count > comma_count {
            b';'
        } else {
            b','
        }
    }

    pub fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean = content.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path)
            .to_string();

        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("csv")
            .to_lowercase();

        let content_type = if ext == "tsv" { "tsv" } else { "csv" };

        let delimiter = Self::detect_delimiter(clean, &ext);

        let mut rdr = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(false)
            .flexible(true)
            .from_reader(clean.as_bytes());

        let mut records: Vec<Vec<String>> = Vec::new();
        for result in rdr.records() {
            match result {
                Ok(rec) => {
                    let fields: Vec<String> = rec.iter().map(|s| s.trim().to_string()).collect();
                    records.push(fields);
                }
                Err(_) => {
                    // Fallback to naive line splitting if csv crate fails on malformed line
                    return Ok(splitter::split_oversized_code(clean, self.max_chunk_chars)
                        .into_iter()
                        .enumerate()
                        .map(|(i, slice)| {
                            let start = slice.start_line_offset + 1;
                            let end = slice.end_line_offset + 1;
                            let header_path = format!("{} > rows {}..{}", file_name, start, end);
                            let text = format!("// Context: {}\n{}", header_path, slice.text);
                            ChunkPayload::new(
                                format!("{}_chunk_{}", file_name, i),
                                text,
                                file_name.clone(),
                                file_path.to_string(),
                                Some(header_path),
                                start,
                                end,
                                content_type.to_string(),
                                i,
                            )
                        })
                        .collect());
                }
            }
        }

        if records.is_empty() {
            return Ok(Vec::new());
        }

        // Row 0 is the headers
        let headers: Vec<String> = records[0].clone();
        let header_summary = format!("[{}]", headers.join(", "));

        // Build Markdown table header
        let table_header_line = format!("| {} |", headers.join(" | "));
        let table_separator_line = format!("|{}|", headers.iter().map(|_| "---").collect::<Vec<&str>>().join("|"));
        let header_block = format!("{}\n{}", table_header_line, table_separator_line);

        // If whole table is small enough, emit a single chunk
        let num_data_rows = records.len().saturating_sub(1);
        if clean.len() <= self.max_chunk_chars {
            let header_path = format!("{} > {} > rows 1..{}", file_name, header_summary, num_data_rows);
            let mut full_table = header_block.clone();
            for row in records.iter().skip(1) {
                full_table.push('\n');
                full_table.push_str(&format!("| {} |", row.join(" | ")));
            }
            let text = format!("// Context: {}\n{}", header_path, full_table);
            return Ok(vec![ChunkPayload::new(
                format!("{}_chunk_0", file_name),
                text,
                file_name,
                file_path.to_string(),
                Some(header_path),
                1,
                records.len(),
                content_type.to_string(),
                0,
            )]);
        }

        // Multi-chunk row-windowing with header propagation
        let mut chunks = Vec::new();
        let mut current_rows: Vec<String> = Vec::new();
        let mut current_start_row = 1;
        let mut current_chars = header_block.len() + 100; // estimated prefix overhead

        for (idx, row) in records.iter().skip(1).enumerate() {
            let row_idx = idx + 1; // 1-indexed data row
            let formatted_row = format!("| {} |", row.join(" | "));
            let row_len = formatted_row.len() + 1;

            if !current_rows.is_empty() && (current_chars + row_len > self.max_chunk_chars) {
                let end_row = row_idx - 1;
                let header_path = format!("{} > {} > rows {}..{}", file_name, header_summary, current_start_row, end_row);
                let table_content = format!("{}\n{}", header_block, current_rows.join("\n"));
                let text = format!("// Context: {}\n{}", header_path, table_content);

                chunks.push(ChunkPayload::new(
                    format!("{}_chunk_{}", file_name, chunks.len()),
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
                current_chars = header_block.len() + 100;
            }

            current_rows.push(formatted_row);
            current_chars += row_len;
        }

        if !current_rows.is_empty() {
            let end_row = num_data_rows;
            let header_path = format!("{} > {} > rows {}..{}", file_name, header_summary, current_start_row, end_row);
            let table_content = format!("{}\n{}", header_block, current_rows.join("\n"));
            let text = format!("// Context: {}\n{}", header_path, table_content);

            chunks.push(ChunkPayload::new(
                format!("{}_chunk_{}", file_name, chunks.len()),
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

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_csv_single_chunk() {
        let chunker = CsvChunker::new(1800);
        let csv = "Name,Age,Role\nAlice,30,Engineer\nBob,25,Designer\n";
        let chunks = chunker.chunk("users.csv", csv).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "csv");
        assert!(chunks[0].text.contains("// Context: users.csv > [Name, Age, Role] > rows 1..2"));
        assert!(chunks[0].text.contains("| Alice | 30 | Engineer |"));
    }

    #[test]
    fn test_tsv_chunking() {
        let chunker = CsvChunker::new(1800);
        let tsv = "ID\tProduct\tPrice\n1\tLaptop\t999.99\n2\tMouse\t29.99\n";
        let chunks = chunker.chunk("catalog.tsv", tsv).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "tsv");
        assert!(chunks[0].text.contains("// Context: catalog.tsv > [ID, Product, Price] > rows 1..2"));
        assert!(chunks[0].text.contains("| 1 | Laptop | 999.99 |"));
    }

    #[test]
    fn test_semicolon_csv_chunking() {
        let chunker = CsvChunker::new(1800);
        let csv = "Codigo;Descricao;Valor\n001;Teclado;150,00\n002;Monitor;800,00\n";
        let chunks = chunker.chunk("produtos.csv", csv).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("| Codigo | Descricao | Valor |"));
        assert!(chunks[0].text.contains("| 001 | Teclado | 150,00 |"));
    }

    #[test]
    fn test_large_csv_header_propagation() {
        let chunker = CsvChunker::new(120);
        let mut csv = String::from("ColA,ColB,ColC\n");
        for i in 1..=20 {
            csv.push_str(&format!("val_a_{},val_b_{},val_c_{}\n", i, i, i));
        }

        let chunks = chunker.chunk("data.csv", &csv).expect("Chunking failed");
        assert!(chunks.len() > 1, "Expected multiple chunks, got {}", chunks.len());

        for chunk in &chunks {
            // Every chunk MUST have the propagated header
            assert!(chunk.text.contains("| ColA | ColB | ColC |"), "Missing header in chunk: {}", chunk.text);
            assert!(chunk.header_path.as_deref().unwrap_or("").contains("[ColA, ColB, ColC]"));
        }
    }
}
