use std::path::Path;
use serde_json::Value;
use crate::ingestion::chunkers::code::splitter::split_oversized_code;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct JsonChunker {
    pub max_chunk_chars: usize,
}

impl Default for JsonChunker {
    fn default() -> Self {
        Self { max_chunk_chars: 1800 }
    }
}

impl JsonChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            max_chunk_chars: if max_chunk_chars == 0 { 1800 } else { max_chunk_chars },
        }
    }

    fn chunk_jsonl(&self, file_path: &str, file_name: &str, content: &str) -> Vec<ChunkPayload> {
        let mut chunks = Vec::new();
        let mut current_lines = Vec::new();
        let mut current_chars = 0usize;
        let mut batch_start_line = 1usize;

        for (line_idx, line) in content.lines().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.len() > self.max_chunk_chars {
                // Flush accumulated batch
                if !current_lines.is_empty() {
                    let batch_end_line = line_num.saturating_sub(1).max(batch_start_line);
                    let header_path = format!("{} > lines {}..{}", file_name, batch_start_line, batch_end_line);
                    let text = format!("// Context: {}\n---\n{}", header_path, current_lines.join("\n"));
                    let chunk_idx = chunks.len();
                    let id = format!("{}_{}_{}", file_name, batch_start_line, chunk_idx);
                    chunks.push(ChunkPayload::new(
                        id,
                        text,
                        file_name.to_string(),
                        file_path.to_string(),
                        Some(header_path),
                        batch_start_line,
                        batch_end_line,
                        "json".to_string(),
                        chunk_idx,
                    ));
                    current_lines.clear();
                    current_chars = 0;
                }

                // Split oversized single line
                let parts = split_oversized_code(trimmed, self.max_chunk_chars);
                for part in parts {
                    let header_path = format!("{} > line {}", file_name, line_num);
                    let text = format!("// Context: {}\n---\n{}", header_path, part.text);
                    let chunk_idx = chunks.len();
                    let id = format!("{}_{}_{}", file_name, line_num, chunk_idx);
                    chunks.push(ChunkPayload::new(
                        id,
                        text,
                        file_name.to_string(),
                        file_path.to_string(),
                        Some(header_path),
                        line_num,
                        line_num,
                        "json".to_string(),
                        chunk_idx,
                    ));
                }
                batch_start_line = line_num + 1;
                continue;
            }

            if current_chars + trimmed.len() + 1 > self.max_chunk_chars && !current_lines.is_empty() {
                let batch_end_line = line_num.saturating_sub(1).max(batch_start_line);
                let header_path = format!("{} > lines {}..{}", file_name, batch_start_line, batch_end_line);
                let text = format!("// Context: {}\n---\n{}", header_path, current_lines.join("\n"));
                let chunk_idx = chunks.len();
                let id = format!("{}_{}_{}", file_name, batch_start_line, chunk_idx);
                chunks.push(ChunkPayload::new(
                    id,
                    text,
                    file_name.to_string(),
                    file_path.to_string(),
                    Some(header_path),
                    batch_start_line,
                    batch_end_line,
                    "json".to_string(),
                    chunk_idx,
                ));
                current_lines.clear();
                current_chars = 0;
                batch_start_line = line_num;
            }

            if current_lines.is_empty() {
                batch_start_line = line_num;
            }
            current_chars += trimmed.len() + 1;
            current_lines.push(trimmed);
        }

        if !current_lines.is_empty() {
            let total_lines = content.lines().count().max(1);
            let header_path = format!("{} > lines {}..{}", file_name, batch_start_line, total_lines);
            let text = format!("// Context: {}\n---\n{}", header_path, current_lines.join("\n"));
            let chunk_idx = chunks.len();
            let id = format!("{}_{}_{}", file_name, batch_start_line, chunk_idx);
            chunks.push(ChunkPayload::new(
                id,
                text,
                file_name.to_string(),
                file_path.to_string(),
                Some(header_path),
                batch_start_line,
                total_lines,
                "json".to_string(),
                chunk_idx,
            ));
        }

        chunks
    }

    fn fallback_chunk(&self, file_path: &str, file_name: &str, content: &str) -> Vec<ChunkPayload> {
        let parts = split_oversized_code(content, self.max_chunk_chars);
        let mut chunks = Vec::new();
        for (idx, part) in parts.into_iter().enumerate() {
            let p_start = 1 + part.start_line_offset;
            let p_end = (1 + part.end_line_offset).max(p_start);
            let header_path = format!("{} > json", file_name);
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
                "json".to_string(),
                idx,
            ));
        }
        chunks
    }
}

impl Chunker for JsonChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean = content.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("document.json")
            .to_string();

        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check if JSON Lines format
        if ext == "jsonl" || ext == "ndjson" {
            return Ok(self.chunk_jsonl(file_path, &file_name, content));
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
                "json".to_string(),
                0,
            )]);
        }

        // Parse JSON structure
        let parsed: Result<Value, _> = serde_json::from_str(content);
        let root_val = match parsed {
            Ok(v) => v,
            Err(_) => return Ok(self.fallback_chunk(file_path, &file_name, content)),
        };

        let mut chunks = Vec::new();

        match root_val {
            Value::Array(items) => {
                self.chunk_array(&items, &file_name, file_path, "root", &mut chunks);
            }
            Value::Object(map) => {
                self.chunk_object(&map, &file_name, file_path, "root", &mut chunks);
            }
            _ => {
                chunks = self.fallback_chunk(file_path, &file_name, content);
            }
        }

        if chunks.is_empty() {
            chunks = self.fallback_chunk(file_path, &file_name, content);
        }

        Ok(chunks)
    }
}

impl JsonChunker {
    fn chunk_array(
        &self,
        items: &[Value],
        file_name: &str,
        file_path: &str,
        path_prefix: &str,
        chunks: &mut Vec<ChunkPayload>,
    ) {
        if items.is_empty() {
            return;
        }

        let mut batch: Vec<String> = Vec::new();
        let mut batch_chars = 0usize;
        let mut batch_start_idx = 0usize;

        let flush_batch = |batch: &mut Vec<String>,
                           batch_chars: &mut usize,
                           start_idx: usize,
                           end_idx: usize,
                           chunks: &mut Vec<ChunkPayload>| {
            if batch.is_empty() {
                return;
            }
            let combined = format!("[\n  {}\n]", batch.join(",\n  "));
            let header_path = format!("{} > {} > items [{}..{}]", file_name, path_prefix, start_idx, end_idx);
            let text = format!("// Context: {}\n---\n{}", header_path, combined);
            let chunk_idx = chunks.len();
            let id = format!("{}_{}_{}", file_name, start_idx + 1, chunk_idx);
            chunks.push(ChunkPayload::new(
                id,
                text,
                file_name.to_string(),
                file_path.to_string(),
                Some(header_path),
                start_idx + 1,
                end_idx + 1,
                "json".to_string(),
                chunk_idx,
            ));
            batch.clear();
            *batch_chars = 0;
        };

        for (idx, item) in items.iter().enumerate() {
            let pretty = serde_json::to_string_pretty(item).unwrap_or_default();
            if pretty.len() > self.max_chunk_chars {
                // Flush previous batch
                let prev_end = idx.saturating_sub(1);
                flush_batch(&mut batch, &mut batch_chars, batch_start_idx, prev_end, chunks);

                // If oversized item is an object, chunk its keys
                if let Value::Object(sub_map) = item {
                    let sub_path = format!("{}[{}]", path_prefix, idx);
                    self.chunk_object(sub_map, file_name, file_path, &sub_path, chunks);
                } else {
                    let parts = split_oversized_code(&pretty, self.max_chunk_chars);
                    for part in parts {
                        let header_path = format!("{} > {} > item[{}]", file_name, path_prefix, idx);
                        let text = format!("// Context: {}\n---\n{}", header_path, part.text);
                        let chunk_idx = chunks.len();
                        let id = format!("{}_{}_{}", file_name, idx + 1, chunk_idx);
                        chunks.push(ChunkPayload::new(
                            id,
                            text,
                            file_name.to_string(),
                            file_path.to_string(),
                            Some(header_path),
                            idx + 1,
                            idx + 1,
                            "json".to_string(),
                            chunk_idx,
                        ));
                    }
                }
                batch_start_idx = idx + 1;
            } else {
                if batch_chars + pretty.len() + 4 > self.max_chunk_chars && !batch.is_empty() {
                    let prev_end = idx.saturating_sub(1);
                    flush_batch(&mut batch, &mut batch_chars, batch_start_idx, prev_end, chunks);
                    batch_start_idx = idx;
                }
                batch_chars += pretty.len() + 4;
                batch.push(pretty);
            }
        }

        if !batch.is_empty() {
            let end_idx = items.len().saturating_sub(1);
            flush_batch(&mut batch, &mut batch_chars, batch_start_idx, end_idx, chunks);
        }
    }

    fn chunk_object(
        &self,
        map: &serde_json::Map<String, Value>,
        file_name: &str,
        file_path: &str,
        path_prefix: &str,
        chunks: &mut Vec<ChunkPayload>,
    ) {
        if map.is_empty() {
            return;
        }

        let mut batch_keys: Vec<String> = Vec::new();
        let mut batch_entries: Vec<String> = Vec::new();
        let mut batch_chars = 0usize;

        let flush_entries = |batch_keys: &mut Vec<String>,
                             batch_entries: &mut Vec<String>,
                             batch_chars: &mut usize,
                             chunks: &mut Vec<ChunkPayload>| {
            if batch_entries.is_empty() {
                return;
            }
            let combined = format!("{{\n  {}\n}}", batch_entries.join(",\n  "));
            let keys_str = if batch_keys.len() <= 3 {
                batch_keys.join(", ")
            } else {
                format!("{}..{} ({} keys)", batch_keys[0], batch_keys[batch_keys.len() - 1], batch_keys.len())
            };
            let header_path = format!("{} > {} > {{{}}}", file_name, path_prefix, keys_str);
            let text = format!("// Context: {}\n---\n{}", header_path, combined);
            let chunk_idx = chunks.len();
            let id = format!("{}_{}_{}", file_name, chunk_idx + 1, chunk_idx);
            chunks.push(ChunkPayload::new(
                id,
                text,
                file_name.to_string(),
                file_path.to_string(),
                Some(header_path),
                chunk_idx + 1,
                chunk_idx + 1,
                "json".to_string(),
                chunk_idx,
            ));
            batch_keys.clear();
            batch_entries.clear();
            *batch_chars = 0;
        };

        for (key, val) in map {
            let val_str = serde_json::to_string_pretty(val).unwrap_or_default();
            let entry = format!("\"{}\": {}", key, val_str.replace('\n', "\n  "));

            if entry.len() > self.max_chunk_chars {
                // Flush previous batch
                flush_entries(&mut batch_keys, &mut batch_entries, &mut batch_chars, chunks);

                // Deeply chunk nested structures
                match val {
                    Value::Array(sub_items) => {
                        let sub_path = format!("{}.{}", path_prefix, key);
                        self.chunk_array(sub_items, file_name, file_path, &sub_path, chunks);
                    }
                    Value::Object(sub_map) => {
                        let sub_path = format!("{}.{}", path_prefix, key);
                        self.chunk_object(sub_map, file_name, file_path, &sub_path, chunks);
                    }
                    _ => {
                        let parts = split_oversized_code(&entry, self.max_chunk_chars);
                        for part in parts {
                            let header_path = format!("{} > {} > {}", file_name, path_prefix, key);
                            let text = format!("// Context: {}\n---\n{}", header_path, part.text);
                            let chunk_idx = chunks.len();
                            let id = format!("{}_{}_{}", file_name, chunk_idx + 1, chunk_idx);
                            chunks.push(ChunkPayload::new(
                                id,
                                text,
                                file_name.to_string(),
                                file_path.to_string(),
                                Some(header_path),
                                chunk_idx + 1,
                                chunk_idx + 1,
                                "json".to_string(),
                                chunk_idx,
                            ));
                        }
                    }
                }
            } else {
                if batch_chars + entry.len() + 4 > self.max_chunk_chars && !batch_entries.is_empty() {
                    flush_entries(&mut batch_keys, &mut batch_entries, &mut batch_chars, chunks);
                }
                batch_chars += entry.len() + 4;
                batch_keys.push(key.clone());
                batch_entries.push(entry);
            }
        }

        flush_entries(&mut batch_keys, &mut batch_entries, &mut batch_chars, chunks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_json_single_chunk() {
        let chunker = JsonChunker::new(1000);
        let json = r#"{"name": "any-context", "version": "0.30.7", "status": "active"}"#;
        let chunks = chunker.chunk("package.json", json).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("// Context: package.json > root"));
        assert!(chunks[0].text.contains("any-context"));
    }

    #[test]
    fn test_jsonl_chunking() {
        let chunker = JsonChunker::new(200);
        let mut jsonl = String::new();
        for i in 1..=10 {
            jsonl.push_str(&format!("{{\"id\": {}, \"event\": \"action_{}\"}}\n", i, i));
        }

        let chunks = chunker.chunk("events.jsonl", &jsonl).expect("Chunking failed");
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.text.contains("// Context: events.jsonl > lines"));
            assert_eq!(chunk.content_type, "json");
        }
    }

    #[test]
    fn test_large_json_object_hierarchical_keys() {
        let chunker = JsonChunker::new(300);
        let mut json = String::from("{\n");
        for i in 1..=10 {
            json.push_str(&format!("  \"config_key_{}\": \"value_data_payload_{}\",\n", i, i));
        }
        json.push_str("  \"final_key\": \"final_val\"\n}");

        let chunks = chunker.chunk("config.json", &json).expect("Chunking failed");
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.text.contains("// Context: config.json > root"));
            assert_eq!(chunk.content_type, "json");
        }
    }
}
