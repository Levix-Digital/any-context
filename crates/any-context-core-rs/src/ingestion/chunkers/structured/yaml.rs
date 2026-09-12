use std::path::Path;
use serde_yaml::Value;
use crate::ingestion::chunkers::code::splitter::split_oversized_code;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct YamlChunker {
    pub max_chunk_chars: usize,
}

impl Default for YamlChunker {
    fn default() -> Self {
        Self { max_chunk_chars: 1800 }
    }
}

impl YamlChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            max_chunk_chars: if max_chunk_chars == 0 { 1800 } else { max_chunk_chars },
        }
    }
}

fn extract_yaml_label(doc: &str) -> Option<String> {
    let mut kind = None;
    let mut name = None;
    for line in doc.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("kind:") && kind.is_none() {
            let val = trimmed.trim_start_matches("kind:").trim().trim_matches('\'').trim_matches('"');
            if !val.is_empty() {
                kind = Some(val.to_string());
            }
        } else if trimmed.starts_with("name:") && name.is_none() {
            let val = trimmed.trim_start_matches("name:").trim().trim_matches('\'').trim_matches('"');
            if !val.is_empty() {
                name = Some(val.to_string());
            }
        }
        if kind.is_some() && name.is_some() {
            break;
        }
    }
    match (kind, name) {
        (Some(k), Some(n)) => Some(format!("{}: {}", k, n)),
        (Some(k), None) => Some(k),
        (None, Some(n)) => Some(n),
        (None, None) => None,
    }
}

struct YamlDoc {
    index: usize,
    text: String,
    start_line: usize,
    end_line: usize,
}

fn split_yaml_documents(content: &str) -> Vec<YamlDoc> {
    let mut docs = Vec::new();
    let mut current_doc = String::new();
    let mut doc_start_line = 1usize;
    let mut doc_idx = 0usize;

    for (line_idx, line) in content.lines().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        if (trimmed == "---" || trimmed.starts_with("--- ") || trimmed.starts_with("---\t")) && !current_doc.trim().is_empty() {
            let end_line = line_num.saturating_sub(1).max(doc_start_line);
            docs.push(YamlDoc {
                index: doc_idx,
                text: current_doc.trim().to_string(),
                start_line: doc_start_line,
                end_line,
            });
            doc_idx += 1;
            current_doc.clear();
            doc_start_line = line_num;
            continue;
        }

        if current_doc.is_empty() && (trimmed == "---" || trimmed.starts_with("--- ") || trimmed.starts_with("---\t")) {
            doc_start_line = line_num;
            continue;
        }

        current_doc.push_str(line);
        current_doc.push('\n');
    }

    if !current_doc.trim().is_empty() {
        let total_lines = content.lines().count().max(1);
        docs.push(YamlDoc {
            index: doc_idx,
            text: current_doc.trim().to_string(),
            start_line: doc_start_line,
            end_line: total_lines,
        });
    }

    docs
}

impl Chunker for YamlChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean = content.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("document.yaml")
            .to_string();

        let docs = split_yaml_documents(content);
        if docs.is_empty() {
            return Ok(self.fallback_chunk(file_path, &file_name, content, 1));
        }

        let is_multi_doc = docs.len() > 1;
        let mut chunks = Vec::new();

        for doc in &docs {
            let label = extract_yaml_label(&doc.text);
            let doc_header_desc = match (&label, is_multi_doc) {
                (Some(lbl), true) => format!("doc {} ({})", doc.index + 1, lbl),
                (Some(lbl), false) => lbl.clone(),
                (None, true) => format!("doc {}", doc.index + 1),
                (None, false) => "root".to_string(),
            };

            // If this document fits in a single chunk
            if doc.text.len() <= self.max_chunk_chars {
                let header_path = format!("{} > {}", file_name, doc_header_desc);
                let text = format!("// Context: {}\n---\n{}", header_path, doc.text);
                let chunk_idx = chunks.len();
                let id = format!("{}_{}_{}", file_name, doc.start_line, chunk_idx);
                chunks.push(ChunkPayload::new(
                    id,
                    text,
                    file_name.clone(),
                    file_path.to_string(),
                    Some(header_path),
                    doc.start_line,
                    doc.end_line,
                    "yaml".to_string(),
                    chunk_idx,
                ));
                continue;
            }

            // Document is oversized. Try parsing YAML structure
            let parsed: Result<Value, _> = serde_yaml::from_str(&doc.text);
            match parsed {
                Ok(Value::Mapping(map)) => {
                    self.chunk_mapping(
                        &map,
                        &file_name,
                        file_path,
                        &doc_header_desc,
                        doc.start_line,
                        doc.end_line,
                        &mut chunks,
                    );
                }
                Ok(Value::Sequence(seq)) => {
                    self.chunk_sequence(
                        &seq,
                        &file_name,
                        file_path,
                        &doc_header_desc,
                        doc.start_line,
                        doc.end_line,
                        &mut chunks,
                    );
                }
                _ => {
                    let parts = split_oversized_code(&doc.text, self.max_chunk_chars);
                    for part in parts {
                        let p_start = doc.start_line + part.start_line_offset;
                        let p_end = (doc.start_line + part.end_line_offset).max(p_start);
                        let header_path = format!("{} > {}", file_name, doc_header_desc);
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
                            "yaml".to_string(),
                            chunk_idx,
                        ));
                    }
                }
            }
        }

        if chunks.is_empty() {
            chunks = self.fallback_chunk(file_path, &file_name, content, 1);
        }

        Ok(chunks)
    }
}

impl YamlChunker {
    fn chunk_mapping(
        &self,
        map: &serde_yaml::Mapping,
        file_name: &str,
        file_path: &str,
        path_prefix: &str,
        start_line: usize,
        _end_line: usize,
        chunks: &mut Vec<ChunkPayload>,
    ) {
        if map.is_empty() {
            return;
        }

        let mut batch_keys: Vec<String> = Vec::new();
        let mut batch_yaml = String::new();

        let flush_batch = |batch_keys: &mut Vec<String>,
                           batch_yaml: &mut String,
                           chunks: &mut Vec<ChunkPayload>| {
            let trimmed = batch_yaml.trim();
            if trimmed.is_empty() {
                return;
            }
            let keys_desc = if batch_keys.len() <= 3 {
                batch_keys.join(", ")
            } else {
                format!("{}..{} ({} keys)", batch_keys[0], batch_keys[batch_keys.len() - 1], batch_keys.len())
            };
            let header_path = format!("{} > {} > {{{}}}", file_name, path_prefix, keys_desc);
            let text = format!("// Context: {}\n---\n{}", header_path, trimmed);
            let chunk_idx = chunks.len();
            let id = format!("{}_{}_{}", file_name, start_line, chunk_idx);
            chunks.push(ChunkPayload::new(
                id,
                text,
                file_name.to_string(),
                file_path.to_string(),
                Some(header_path),
                start_line,
                start_line + trimmed.lines().count().saturating_sub(1),
                "yaml".to_string(),
                chunk_idx,
            ));
            batch_keys.clear();
            batch_yaml.clear();
        };

        for (k_val, v_val) in map {
            let key_str = match k_val {
                Value::String(s) => s.clone(),
                _ => format!("{:?}", k_val),
            };

            let mut single_map = serde_yaml::Mapping::new();
            single_map.insert(k_val.clone(), v_val.clone());
            let entry_yaml = serde_yaml::to_string(&single_map).unwrap_or_default();

            if entry_yaml.len() > self.max_chunk_chars {
                // Flush previous batch
                flush_batch(&mut batch_keys, &mut batch_yaml, chunks);

                if let Value::Mapping(sub_map) = v_val {
                    let sub_path = format!("{}.{}", path_prefix, key_str);
                    self.chunk_mapping(sub_map, file_name, file_path, &sub_path, start_line, _end_line, chunks);
                } else if let Value::Sequence(sub_seq) = v_val {
                    let sub_path = format!("{}.{}", path_prefix, key_str);
                    self.chunk_sequence(sub_seq, file_name, file_path, &sub_path, start_line, _end_line, chunks);
                } else {
                    let parts = split_oversized_code(&entry_yaml, self.max_chunk_chars);
                    for part in parts {
                        let header_path = format!("{} > {} > {}", file_name, path_prefix, key_str);
                        let text = format!("// Context: {}\n---\n{}", header_path, part.text);
                        let chunk_idx = chunks.len();
                        let id = format!("{}_{}_{}", file_name, start_line, chunk_idx);
                        chunks.push(ChunkPayload::new(
                            id,
                            text,
                            file_name.to_string(),
                            file_path.to_string(),
                            Some(header_path),
                            start_line,
                            start_line + part.text.lines().count().saturating_sub(1),
                            "yaml".to_string(),
                            chunk_idx,
                        ));
                    }
                }
            } else {
                if batch_yaml.len() + entry_yaml.len() > self.max_chunk_chars && !batch_yaml.is_empty() {
                    flush_batch(&mut batch_keys, &mut batch_yaml, chunks);
                }
                batch_yaml.push_str(&entry_yaml);
                batch_keys.push(key_str);
            }
        }

        flush_batch(&mut batch_keys, &mut batch_yaml, chunks);
    }

    fn chunk_sequence(
        &self,
        seq: &[Value],
        file_name: &str,
        file_path: &str,
        path_prefix: &str,
        start_line: usize,
        _end_line: usize,
        chunks: &mut Vec<ChunkPayload>,
    ) {
        if seq.is_empty() {
            return;
        }

        let mut batch_items: Vec<String> = Vec::new();
        let mut batch_chars = 0usize;
        let mut batch_start_idx = 0usize;

        let flush_items = |batch_items: &mut Vec<String>,
                           batch_chars: &mut usize,
                           start_idx: usize,
                           end_idx: usize,
                           chunks: &mut Vec<ChunkPayload>| {
            if batch_items.is_empty() {
                return;
            }
            let combined = batch_items.join("\n");
            let header_path = format!("{} > {} > items [{}..{}]", file_name, path_prefix, start_idx, end_idx);
            let text = format!("// Context: {}\n---\n{}", header_path, combined);
            let chunk_idx = chunks.len();
            let id = format!("{}_{}_{}", file_name, start_line, chunk_idx);
            chunks.push(ChunkPayload::new(
                id,
                text,
                file_name.to_string(),
                file_path.to_string(),
                Some(header_path),
                start_line,
                start_line + combined.lines().count().saturating_sub(1),
                "yaml".to_string(),
                chunk_idx,
            ));
            batch_items.clear();
            *batch_chars = 0;
        };

        for (idx, item) in seq.iter().enumerate() {
            let item_yaml = serde_yaml::to_string(item).unwrap_or_default();
            if item_yaml.len() > self.max_chunk_chars {
                let prev_end = idx.saturating_sub(1);
                flush_items(&mut batch_items, &mut batch_chars, batch_start_idx, prev_end, chunks);

                if let Value::Mapping(sub_map) = item {
                    let sub_path = format!("{}[{}]", path_prefix, idx);
                    self.chunk_mapping(sub_map, file_name, file_path, &sub_path, start_line, _end_line, chunks);
                } else {
                    let parts = split_oversized_code(&item_yaml, self.max_chunk_chars);
                    for part in parts {
                        let header_path = format!("{} > {} > item[{}]", file_name, path_prefix, idx);
                        let text = format!("// Context: {}\n---\n{}", header_path, part.text);
                        let chunk_idx = chunks.len();
                        let id = format!("{}_{}_{}", file_name, start_line, chunk_idx);
                        chunks.push(ChunkPayload::new(
                            id,
                            text,
                            file_name.to_string(),
                            file_path.to_string(),
                            Some(header_path),
                            start_line,
                            start_line + part.text.lines().count().saturating_sub(1),
                            "yaml".to_string(),
                            chunk_idx,
                        ));
                    }
                }
                batch_start_idx = idx + 1;
            } else {
                if batch_chars + item_yaml.len() > self.max_chunk_chars && !batch_items.is_empty() {
                    let prev_end = idx.saturating_sub(1);
                    flush_items(&mut batch_items, &mut batch_chars, batch_start_idx, prev_end, chunks);
                    batch_start_idx = idx;
                }
                batch_chars += item_yaml.len();
                batch_items.push(item_yaml.trim().to_string());
            }
        }

        if !batch_items.is_empty() {
            let end_idx = seq.len().saturating_sub(1);
            flush_items(&mut batch_items, &mut batch_chars, batch_start_idx, end_idx, chunks);
        }
    }

    fn fallback_chunk(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        start_line: usize,
    ) -> Vec<ChunkPayload> {
        let parts = split_oversized_code(content, self.max_chunk_chars);
        let mut chunks = Vec::new();
        for (idx, part) in parts.into_iter().enumerate() {
            let p_start = start_line + part.start_line_offset;
            let p_end = (start_line + part.end_line_offset).max(p_start);
            let header_path = format!("{} > yaml", file_name);
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
                "yaml".to_string(),
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
    fn test_small_yaml_single_chunk() {
        let chunker = YamlChunker::new(1000);
        let yaml = "version: '3.8'\nservices:\n  web:\n    image: nginx:latest\n    ports:\n      - '80:80'\n";
        let chunks = chunker.chunk("docker-compose.yml", yaml).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("// Context: docker-compose.yml"));
        assert!(chunks[0].text.contains("nginx:latest"));
        assert_eq!(chunks[0].content_type, "yaml");
    }

    #[test]
    fn test_multi_document_yaml() {
        let chunker = YamlChunker::new(1000);
        let yaml = r#"apiVersion: v1
kind: Service
metadata:
  name: frontend-svc
spec:
  ports:
    - port: 80
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: frontend-dep
spec:
  replicas: 3
"#;
        let chunks = chunker.chunk("manifest.yaml", yaml).expect("Chunking failed");
        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].text.contains("Service: frontend-svc"));
        assert!(chunks[1].text.contains("Deployment: frontend-dep"));
        assert_eq!(chunks[0].content_type, "yaml");
        assert_eq!(chunks[1].content_type, "yaml");
    }

    #[test]
    fn test_large_yaml_mapping() {
        let chunker = YamlChunker::new(200);
        let mut yaml = String::from("settings:\n");
        for i in 1..=10 {
            yaml.push_str(&format!("  setting_key_{}: \"value_config_{}\"\n", i, i));
        }

        let chunks = chunker.chunk("config.yaml", &yaml).expect("Chunking failed");
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.text.contains("// Context: config.yaml"));
            assert_eq!(chunk.content_type, "yaml");
        }
    }
}
