use std::path::Path;
use crate::ingestion::chunkers::code::splitter::split_oversized_code;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct TomlChunker {
    pub max_chunk_chars: usize,
}

impl Default for TomlChunker {
    fn default() -> Self {
        Self { max_chunk_chars: 1800 }
    }
}

impl TomlChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            max_chunk_chars: if max_chunk_chars == 0 { 1800 } else { max_chunk_chars },
        }
    }
}

#[derive(Debug)]
struct TomlSection {
    header: String,
    text: String,
    start_line: usize,
    end_line: usize,
}

fn extract_table_name(line: &str) -> Option<(String, bool)> {
    let trimmed = line.trim();
    if trimmed.starts_with("#") {
        return None;
    }

    // Strip trailing comment if present outside quotes
    let clean = if let Some(hash_idx) = trimmed.find('#') {
        let before_hash = &trimmed[..hash_idx].trim_end();
        if (before_hash.starts_with("[[") && before_hash.ends_with("]]"))
            || (before_hash.starts_with('[') && before_hash.ends_with(']'))
        {
            before_hash
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    if clean.starts_with("[[") && clean.ends_with("]]") && clean.len() >= 4 {
        let name = clean[2..clean.len() - 2].trim().to_string();
        if !name.is_empty() {
            return Some((format!("[[{}]]", name), true));
        }
    } else if clean.starts_with('[') && clean.ends_with(']') && clean.len() >= 2 {
        let name = clean[1..clean.len() - 1].trim().to_string();
        if !name.is_empty() {
            return Some((format!("[{}]", name), false));
        }
    }

    None
}

fn extract_resource_name(section_text: &str) -> Option<String> {
    for line in section_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") {
            let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
            if parts.len() == 2 && parts[0].trim() == "name" {
                let val = parts[1].trim().trim_matches('"').trim_matches('\'').trim();
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

fn split_toml_sections(content: &str) -> Vec<TomlSection> {
    let mut sections = Vec::new();
    let mut current_header = "root".to_string();
    let mut current_lines = Vec::new();
    let mut current_start_line = 1usize;

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        if let Some((new_header, _is_array)) = extract_table_name(line) {
            // Flush preceding section if it has non-empty text
            let section_text = current_lines.join("\n").trim().to_string();
            if !section_text.is_empty() {
                let end_line = line_num.saturating_sub(1).max(current_start_line);
                sections.push(TomlSection {
                    header: current_header.clone(),
                    text: section_text,
                    start_line: current_start_line,
                    end_line,
                });
            }
            current_header = new_header;
            current_lines.clear();
            current_start_line = line_num;
            current_lines.push(line);
        } else {
            current_lines.push(line);
        }
    }

    let final_text = current_lines.join("\n").trim().to_string();
    if !final_text.is_empty() {
        let total_lines = content.lines().count().max(1);
        sections.push(TomlSection {
            header: current_header,
            text: final_text,
            start_line: current_start_line,
            end_line: total_lines,
        });
    }

    sections
}

impl Chunker for TomlChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean = content.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("config.toml")
            .to_string();

        // Validate TOML syntax (fallback if completely malformed)
        if toml::from_str::<toml::Value>(content).is_err() {
            return Ok(self.fallback_chunk(file_path, &file_name, content));
        }

        // Single chunk if small enough
        if clean.len() <= self.max_chunk_chars {
            let total_lines = clean.lines().count().max(1);
            let header_path = format!("{} > root", file_name);
            let text = format!("// Context: {}\n---\n{}", header_path, clean);
            return Ok(vec![ChunkPayload::new(
                format!("{}_1_0", file_name),
                text,
                file_name,
                file_path.to_string(),
                Some(header_path),
                1,
                total_lines,
                "toml".to_string(),
                0,
            )]);
        }

        let sections = split_toml_sections(content);
        if sections.is_empty() {
            return Ok(self.fallback_chunk(file_path, &file_name, content));
        }

        let mut chunks = Vec::new();

        for section in sections {
            let label = if let Some(res_name) = extract_resource_name(&section.text) {
                if section.header.starts_with("[[") {
                    format!("{}: {}", section.header, res_name)
                } else {
                    section.header.clone()
                }
            } else {
                section.header.clone()
            };

            if section.text.len() <= self.max_chunk_chars {
                let header_path = format!("{} > {}", file_name, label);
                let text = format!("// Context: {}\n---\n{}", header_path, section.text);
                let chunk_idx = chunks.len();
                let id = format!("{}_{}_{}", file_name, section.start_line, chunk_idx);
                chunks.push(ChunkPayload::new(
                    id,
                    text,
                    file_name.clone(),
                    file_path.to_string(),
                    Some(header_path),
                    section.start_line,
                    section.end_line,
                    "toml".to_string(),
                    chunk_idx,
                ));
            } else {
                // Section is oversized: group lines or split with split_oversized_code
                let parts = split_oversized_code(&section.text, self.max_chunk_chars);
                for part in parts {
                    let p_start = section.start_line + part.start_line_offset;
                    let p_end = (section.start_line + part.end_line_offset).max(p_start);
                    let header_path = format!("{} > {}", file_name, label);
                    let text = format!("// Context: {}\n---\n{}", header_path, part.text);
                    let chunk_idx = chunks.len();
                    let id = format!("{}_{}_{}", file_name, p_start, chunk_idx);
                    chunks.push(ChunkPayload::new(
                        id,
                        text,
                        file_name.clone(),
                        file_path.to_string(),
                        Some(header_path),
                        p_start,
                        p_end,
                        "toml".to_string(),
                        chunk_idx,
                    ));
                }
            }
        }

        if chunks.is_empty() {
            chunks = self.fallback_chunk(file_path, &file_name, content);
        }

        Ok(chunks)
    }
}

impl TomlChunker {
    fn fallback_chunk(&self, file_path: &str, file_name: &str, content: &str) -> Vec<ChunkPayload> {
        let parts = split_oversized_code(content, self.max_chunk_chars);
        let mut chunks = Vec::new();
        for (idx, part) in parts.into_iter().enumerate() {
            let p_start = 1 + part.start_line_offset;
            let p_end = (1 + part.end_line_offset).max(p_start);
            let header_path = format!("{} > toml", file_name);
            let text = format!("// Context: {}\n---\n{}", header_path, part.text);
            let id = format!("{}_{}_{}", file_name, p_start, idx);
            chunks.push(ChunkPayload::new(
                id,
                text,
                file_name.to_string(),
                file_path.to_string(),
                Some(header_path),
                p_start,
                p_end,
                "toml".to_string(),
                idx,
            ));
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_toml_single_chunk() {
        let chunker = TomlChunker::new(1000);
        let toml = r#"
[package]
name = "any-context"
version = "0.30.8"
edition = "2021"
"#;
        let chunks = chunker.chunk("Cargo.toml", toml).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("// Context: Cargo.toml > root"));
        assert!(chunks[0].text.contains("any-context"));
        assert_eq!(chunks[0].content_type, "toml");
    }

    #[test]
    fn test_multi_table_toml() {
        let chunker = TomlChunker::new(150);
        let toml = r#"
[package]
name = "demo-app"
version = "1.0.0"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[features]
default = ["std"]
std = []
"#;
        let chunks = chunker.chunk("Cargo.toml", toml).expect("Chunking failed");
        assert_eq!(chunks.len(), 3);
        assert!(chunks[0].text.contains("[package]"));
        assert!(chunks[1].text.contains("[dependencies]"));
        assert!(chunks[2].text.contains("[features]"));
        for c in &chunks {
            assert_eq!(c.content_type, "toml");
        }
    }

    #[test]
    fn test_array_of_tables_with_resource_name() {
        let chunker = TomlChunker::new(100);
        let toml = r#"
[[bin]]
name = "actx"
path = "src/bin/main.rs"

[[bin]]
name = "actx-server"
path = "src/bin/server.rs"
"#;
        let chunks = chunker.chunk("Cargo.toml", toml).expect("Chunking failed");
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.contains("[[bin]]: actx"));
        assert!(chunks[1].text.contains("[[bin]]: actx-server"));
    }

    #[test]
    fn test_malformed_toml_fallback() {
        let chunker = TomlChunker::new(100);
        let malformed = "this is not valid toml = = = [[[";
        let chunks = chunker.chunk("invalid.toml", malformed).expect("Fallback should succeed");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "toml");
    }
}
