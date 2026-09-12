use std::path::Path;
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

#[derive(Debug, Clone)]
pub struct MarkdownHeaderChunker {
    pub max_chunk_chars: usize,
    pub overlap_chars: usize,
}

impl Default for MarkdownHeaderChunker {
    fn default() -> Self {
        Self {
            max_chunk_chars: 1800,
            overlap_chars: 200,
        }
    }
}

impl MarkdownHeaderChunker {
    pub fn new(max_chunk_chars: usize, overlap_chars: usize) -> Self {
        Self {
            max_chunk_chars: if max_chunk_chars == 0 { 1800 } else { max_chunk_chars },
            overlap_chars,
        }
    }
}

#[derive(Debug, Clone)]
struct HeaderItem {
    level: u32,
    title: String,
}

impl Chunker for MarkdownHeaderChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean_content = content.trim();
        if clean_content.is_empty() {
            return Ok(Vec::new());
        }

        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown.md")
            .to_string();

        let mut chunks: Vec<ChunkPayload> = Vec::new();
        let mut header_stack: Vec<HeaderItem> = Vec::new();

        // Line offset mapping for calculating start_line and end_line
        let lines: Vec<&str> = content.lines().collect();

        // Section accumulator
        let mut current_section_text = String::new();
        let current_section_start_line = 1usize;
        let mut in_heading = false;
        let mut heading_text_buffer = String::new();
        let mut heading_level = 1u32;

        let parser = Parser::new(content);

        // Helper to flush accumulated section text into one or more ChunkPayloads
        let flush_section = |chunks: &mut Vec<ChunkPayload>,
                             section_text: &str,
                             headers: &[HeaderItem],
                             start_line: usize,
                             max_chars: usize,
                             f_name: &str,
                             f_path: &str| {
            let trimmed = section_text.trim();
            if trimmed.is_empty() {
                return;
            }

            let header_path = if !headers.is_empty() {
                let path_str = headers
                    .iter()
                    .map(|h| format!("{} {}", "#".repeat(h.level as usize), h.title))
                    .collect::<Vec<_>>()
                    .join(" > ");
                Some(path_str)
            } else {
                None
            };

            // Estimate end line
            let line_count = trimmed.lines().count();
            let end_line = start_line + line_count.saturating_sub(1);

            // If section fits in a single chunk
            if trimmed.len() <= max_chars {
                let chunk_idx = chunks.len();
                let context_header = match &header_path {
                    Some(hp) => format!("// Context: {}\n---\n{}", hp, trimmed),
                    None => trimmed.to_string(),
                };
                let id = format!("{}_{}_{}", f_name, start_line, chunk_idx);
                chunks.push(ChunkPayload::new(
                    id,
                    context_header,
                    f_name.to_string(),
                    f_path.to_string(),
                    header_path,
                    start_line,
                    end_line,
                    "markdown".to_string(),
                    chunk_idx,
                ));
            } else {
                // Section exceeds max_chars: split by paragraphs (\n\n)
                let paragraphs: Vec<&str> = trimmed.split("\n\n").collect();
                let mut sub_chunk_text = String::new();
                let mut sub_start_line = start_line;

                for p in paragraphs {
                    let p_clean = p.trim();
                    if p_clean.is_empty() {
                        continue;
                    }

                    if !sub_chunk_text.is_empty() && (sub_chunk_text.len() + p_clean.len() + 2 > max_chars) {
                        let chunk_idx = chunks.len();
                        let sub_end_line = sub_start_line + sub_chunk_text.lines().count().saturating_sub(1);
                        let context_header = match &header_path {
                            Some(hp) => format!("// Context: {}\n---\n{}", hp, sub_chunk_text.trim()),
                            None => sub_chunk_text.trim().to_string(),
                        };
                        let id = format!("{}_{}_{}", f_name, sub_start_line, chunk_idx);
                        chunks.push(ChunkPayload::new(
                            id,
                            context_header,
                            f_name.to_string(),
                            f_path.to_string(),
                            header_path.clone(),
                            sub_start_line,
                            sub_end_line,
                            "markdown".to_string(),
                            chunk_idx,
                        ));

                        sub_start_line = sub_end_line + 1;
                        sub_chunk_text.clear();
                    }

                    if !sub_chunk_text.is_empty() {
                        sub_chunk_text.push_str("\n\n");
                    }
                    sub_chunk_text.push_str(p_clean);
                }

                if !sub_chunk_text.trim().is_empty() {
                    let chunk_idx = chunks.len();
                    let sub_end_line = sub_start_line + sub_chunk_text.lines().count().saturating_sub(1);
                    let context_header = match &header_path {
                        Some(hp) => format!("// Context: {}\n---\n{}", hp, sub_chunk_text.trim()),
                        None => sub_chunk_text.trim().to_string(),
                    };
                    let id = format!("{}_{}_{}", f_name, sub_start_line, chunk_idx);
                    chunks.push(ChunkPayload::new(
                        id,
                        context_header,
                        f_name.to_string(),
                        f_path.to_string(),
                        header_path,
                        sub_start_line,
                        sub_end_line,
                        "markdown".to_string(),
                        chunk_idx,
                    ));
                }
            }
        };

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    // 1. Flush previous section
                    flush_section(
                        &mut chunks,
                        &current_section_text,
                        &header_stack,
                        current_section_start_line,
                        self.max_chunk_chars,
                        &file_name,
                        file_path,
                    );
                    current_section_text.clear();

                    in_heading = true;
                    heading_text_buffer.clear();
                    heading_level = level as u32;
                }
                Event::End(TagEnd::Heading(_)) => {
                    in_heading = false;
                    let title = heading_text_buffer.trim().to_string();

                    // Adjust header stack according to heading level
                    while let Some(last) = header_stack.last() {
                        if last.level >= heading_level {
                            header_stack.pop();
                        } else {
                            break;
                        }
                    }
                    header_stack.push(HeaderItem {
                        level: heading_level,
                        title,
                    });

                    // Current section text starts fresh under this heading
                    current_section_text.clear();
                }
                Event::Text(t) => {
                    if in_heading {
                        heading_text_buffer.push_str(&t);
                    } else {
                        current_section_text.push_str(&t);
                    }
                }
                Event::Code(c) => {
                    if in_heading {
                        heading_text_buffer.push_str(&c);
                    } else {
                        current_section_text.push_str(&format!("`{}`", c));
                    }
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    current_section_text.push_str("\n```");
                    match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                            current_section_text.push_str(&lang);
                        }
                        _ => {}
                    }
                    current_section_text.push('\n');
                }
                Event::End(TagEnd::CodeBlock) => {
                    current_section_text.push_str("\n```\n");
                }
                Event::Start(Tag::Item) => {
                    current_section_text.push_str("\n- ");
                }
                Event::End(TagEnd::Item) => {
                    current_section_text.push('\n');
                }
                Event::HardBreak | Event::SoftBreak => {
                    if in_heading {
                        heading_text_buffer.push(' ');
                    } else {
                        current_section_text.push('\n');
                    }
                }
                _ => {}
            }
        }

        // Final flush of remaining section text
        flush_section(
            &mut chunks,
            &current_section_text,
            &header_stack,
            current_section_start_line,
            self.max_chunk_chars,
            &file_name,
            file_path,
        );

        // If no chunks were generated (e.g. pure raw text without recognized events)
        if chunks.is_empty() && !clean_content.is_empty() {
            let id = format!("{}_1_0", file_name);
            chunks.push(ChunkPayload::new(
                id,
                clean_content.to_string(),
                file_name,
                file_path.to_string(),
                None,
                1,
                lines.len(),
                "markdown".to_string(),
                0,
            ));
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_chunker_basic_hierarchy() {
        let md = r#"# Architecture
Overview of the core system.

## Database Layer
The database layer uses LanceDB and SQLite.
It is thread-safe and fast.

### Columnar Storage
LanceDB uses Apache Arrow format.
"#;
        let chunker = MarkdownHeaderChunker::default();
        let chunks = chunker.chunk("docs/arch.md", md).expect("Chunking failed");

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].header_path.as_deref(), Some("# Architecture"));
        assert!(chunks[0].text.contains("Overview of the core system."));

        assert_eq!(chunks[1].header_path.as_deref(), Some("# Architecture > ## Database Layer"));
        assert!(chunks[1].text.contains("uses LanceDB and SQLite"));

        assert_eq!(chunks[2].header_path.as_deref(), Some("# Architecture > ## Database Layer > ### Columnar Storage"));
        assert!(chunks[2].text.contains("uses Apache Arrow format"));
    }

    #[test]
    fn test_markdown_code_block_with_hashes_not_heading() {
        let md = r#"# Code Examples
Here is some python code:

```python
# This is a comment, not a markdown heading
def authenticate():
    pass
```
"#;
        let chunker = MarkdownHeaderChunker::default();
        let chunks = chunker.chunk("docs/code.md", md).expect("Chunking failed");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].header_path.as_deref(), Some("# Code Examples"));
        assert!(chunks[0].text.contains("# This is a comment, not a markdown heading"));
    }

    #[test]
    fn test_empty_content_returns_empty() {
        let chunker = MarkdownHeaderChunker::default();
        let chunks = chunker.chunk("empty.md", "   \n\n  ").expect("Chunking failed");
        assert!(chunks.is_empty());
    }
}
