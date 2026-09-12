use std::path::Path;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use crate::ingestion::chunkers::code::splitter::split_oversized_code;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct XmlChunker {
    pub max_chunk_chars: usize,
}

impl Default for XmlChunker {
    fn default() -> Self {
        Self { max_chunk_chars: 1800 }
    }
}

impl XmlChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            max_chunk_chars: if max_chunk_chars == 0 { 1800 } else { max_chunk_chars },
        }
    }
}

#[derive(Debug, Clone)]
struct XmlElement {
    tag_name: String,
    path: String,
    start_byte: usize,
    end_byte: usize,
    children: Vec<XmlElement>,
}

#[derive(Debug)]
struct OpenTag {
    tag_name: String,
    start_byte: usize,
    children: Vec<XmlElement>,
}

fn byte_to_line(line_starts: &[usize], pos: usize) -> usize {
    match line_starts.binary_search(&pos) {
        Ok(idx) => idx + 1,
        Err(idx) => idx,
    }
}

fn build_line_starts(content: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(content.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

impl Chunker for XmlChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean = content.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("document.xml")
            .to_string();

        let line_starts = build_line_starts(content);

        // Parse XML hierarchy
        let mut reader = Reader::from_str(content);
        let mut stack: Vec<OpenTag> = Vec::new();
        let mut roots: Vec<XmlElement> = Vec::new();
        let mut tag_names: Vec<String> = Vec::new();

        let mut prev_pos = 0usize;
        let mut parse_failed = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let current_pos = reader.buffer_position() as usize;
                    let sub = if current_pos <= content.len() && prev_pos <= current_pos {
                        &content[prev_pos..current_pos]
                    } else {
                        ""
                    };
                    let start_byte = prev_pos + sub.find('<').unwrap_or(0);
                    tag_names.push(tag_name.clone());
                    stack.push(OpenTag {
                        tag_name,
                        start_byte,
                        children: Vec::new(),
                    });
                }
                Ok(Event::End(_)) => {
                    if let Some(open) = stack.pop() {
                        tag_names.pop();
                        let end_byte = (reader.buffer_position() as usize).min(content.len());
                        let path = if tag_names.is_empty() {
                            open.tag_name.clone()
                        } else {
                            format!("{} > {}", tag_names.join(" > "), open.tag_name)
                        };
                        let elem = XmlElement {
                            tag_name: open.tag_name,
                            path,
                            start_byte: open.start_byte,
                            end_byte,
                            children: open.children,
                        };
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(elem);
                        } else {
                            roots.push(elem);
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let current_pos = reader.buffer_position() as usize;
                    let sub = if current_pos <= content.len() && prev_pos <= current_pos {
                        &content[prev_pos..current_pos]
                    } else {
                        ""
                    };
                    let start_byte = prev_pos + sub.find('<').unwrap_or(0);
                    let end_byte = current_pos.min(content.len());
                    let path = if tag_names.is_empty() {
                        tag_name.clone()
                    } else {
                        format!("{} > {}", tag_names.join(" > "), tag_name)
                    };
                    let elem = XmlElement {
                        tag_name,
                        path,
                        start_byte,
                        end_byte,
                        children: Vec::new(),
                    };
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(elem);
                    } else {
                        roots.push(elem);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => {
                    parse_failed = true;
                    break;
                }
                _ => {}
            }
            prev_pos = reader.buffer_position() as usize;
        }

        // Fallback for malformed XML or if no root tags found
        if parse_failed || roots.is_empty() {
            return self.fallback_chunk(file_path, &file_name, content, &line_starts);
        }

        let mut chunks: Vec<ChunkPayload> = Vec::new();
        for root in &roots {
            self.collect_element_chunks(root, content, &file_name, file_path, &line_starts, &mut chunks);
        }

        if chunks.is_empty() {
            return self.fallback_chunk(file_path, &file_name, content, &line_starts);
        }

        Ok(chunks)
    }
}

impl XmlChunker {
    fn collect_element_chunks(
        &self,
        elem: &XmlElement,
        content: &str,
        file_name: &str,
        file_path: &str,
        line_starts: &[usize],
        chunks: &mut Vec<ChunkPayload>,
    ) {
        if elem.start_byte >= elem.end_byte || elem.end_byte > content.len() {
            return;
        }

        let snippet = &content[elem.start_byte..elem.end_byte];
        let trimmed = snippet.trim();

        // If this element fits entirely within max_chunk_chars, emit as a single chunk
        if trimmed.len() <= self.max_chunk_chars {
            let start_line = byte_to_line(line_starts, elem.start_byte);
            let end_line = byte_to_line(line_starts, elem.end_byte.saturating_sub(1)).max(start_line);
            let header_path = format!("{} > {}", file_name, elem.path);
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
                end_line,
                "xml".to_string(),
                chunk_idx,
            ));
            return;
        }

        // Element is larger than max_chunk_chars
        if elem.children.is_empty() {
            // Leaf element with huge text/cdata: split with split_oversized_code
            let start_line = byte_to_line(line_starts, elem.start_byte);
            let parts = split_oversized_code(trimmed, self.max_chunk_chars);
            for part in parts {
                let p_start = start_line + part.start_line_offset;
                let p_end = (start_line + part.end_line_offset).max(p_start);
                let header_path = format!("{} > {}", file_name, elem.path);
                let text = format!("// Context: {}\n---\n{}", header_path, part.text);
                let chunk_idx = chunks.len();
                let id = format!("{}_{}_{}", file_name, p_start, chunk_idx);
                chunks.push(ChunkPayload::new(
                    id,
                    text,
                    file_name.to_string(),
                    file_path.to_string(),
                    Some(header_path),
                    p_start,
                    p_end,
                    "xml".to_string(),
                    chunk_idx,
                ));
            }
            return;
        }

        // Element has children: group smaller children, recurse into oversized ones
        let mut group: Vec<&XmlElement> = Vec::new();
        let mut group_chars: usize = 0;

        let flush_group = |group: &mut Vec<&XmlElement>,
                           group_chars: &mut usize,
                           chunks: &mut Vec<ChunkPayload>| {
            if group.is_empty() {
                return;
            }
            let first = group[0];
            let last = group[group.len() - 1];
            let start_line = byte_to_line(line_starts, first.start_byte);
            let end_line = byte_to_line(line_starts, last.end_byte.saturating_sub(1)).max(start_line);

            let combined: String = group
                .iter()
                .filter_map(|c| {
                    if c.start_byte < c.end_byte && c.end_byte <= content.len() {
                        Some(content[c.start_byte..c.end_byte].trim())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            if !combined.is_empty() {
                let header_path = if group.len() == 1 {
                    format!("{} > {}", file_name, first.path)
                } else {
                    format!("{} > {} > [{}..{}]", file_name, elem.path, first.tag_name, last.tag_name)
                };
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
                    end_line,
                    "xml".to_string(),
                    chunk_idx,
                ));
            }

            group.clear();
            *group_chars = 0;
        };

        for child in &elem.children {
            let child_len = child.end_byte.saturating_sub(child.start_byte);
            if child_len > self.max_chunk_chars {
                flush_group(&mut group, &mut group_chars, chunks);
                self.collect_element_chunks(child, content, file_name, file_path, line_starts, chunks);
            } else {
                if group_chars + child_len + 1 > self.max_chunk_chars && !group.is_empty() {
                    flush_group(&mut group, &mut group_chars, chunks);
                }
                group_chars += child_len + 1;
                group.push(child);
            }
        }

        flush_group(&mut group, &mut group_chars, chunks);
    }

    fn fallback_chunk(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        _line_starts: &[usize],
    ) -> Result<Vec<ChunkPayload>, String> {
        let parts = split_oversized_code(content, self.max_chunk_chars);
        let mut chunks = Vec::new();
        for (idx, part) in parts.into_iter().enumerate() {
            let p_start = 1 + part.start_line_offset;
            let p_end = (1 + part.end_line_offset).max(p_start);
            let header_path = format!("{} > xml", file_name);
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
                "xml".to_string(),
                idx,
            ));
        }
        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_xml_single_chunk() {
        let chunker = XmlChunker::new(1000);
        let xml = "<project><name>AnyContext</name><version>1.0</version></project>";
        let chunks = chunker.chunk("pom.xml", xml).expect("Chunking failed");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("// Context: pom.xml > project"));
        assert!(chunks[0].text.contains("<name>AnyContext</name>"));
    }

    #[test]
    fn test_large_xml_hierarchical_children() {
        let chunker = XmlChunker::new(300);
        let mut xml = String::from("<catalog>\n");
        for i in 1..=10 {
            xml.push_str(&format!(
                "  <book id=\"{}\"><title>Book Number {}</title><author>Author {}</author></book>\n",
                i, i, i
            ));
        }
        xml.push_str("</catalog>");

        let chunks = chunker.chunk("books.xml", &xml).expect("Chunking failed");
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.text.len() <= 600);
            assert!(chunk.text.contains("// Context: books.xml"));
            assert_eq!(chunk.content_type, "xml");
        }
    }

    #[test]
    fn test_malformed_xml_graceful_fallback() {
        let chunker = XmlChunker::new(200);
        let malformed = "<root><unclosed><item>123</root>";
        let chunks = chunker.chunk("bad.xml", malformed).expect("Fallback should succeed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "xml");
    }
}
