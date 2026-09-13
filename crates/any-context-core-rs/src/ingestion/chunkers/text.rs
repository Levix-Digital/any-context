use std::path::Path;
use crate::ingestion::traits::Chunker;
use crate::models::ChunkPayload;

/// Universal Native Text, Script, Configuration and Web Chunker.
/// Handles non-AST plain text files (.txt, .log), shell scripts (.sh, .ps1, .bat),
/// database scripts (.sql), configuration files (.env, .ini, .cfg, .properties),
/// container/build files (Dockerfile, Makefile), and Web Pages.
#[derive(Debug, Clone)]
pub struct TextChunker {
    pub max_chunk_chars: usize,
    pub overlap_chars: usize,
}

impl Default for TextChunker {
    fn default() -> Self {
        Self::new(1800, 200)
    }
}

impl TextChunker {
    pub fn new(max_chunk_chars: usize, overlap_chars: usize) -> Self {
        Self {
            max_chunk_chars: if max_chunk_chars == 0 { 1800 } else { max_chunk_chars },
            overlap_chars: if overlap_chars >= max_chunk_chars && max_chunk_chars > 0 {
                max_chunk_chars / 4
            } else {
                overlap_chars
            },
        }
    }

    /// Checks whether the extension is supported by TextChunker.
    pub fn supports_extension(&self, ext: &str) -> bool {
        matches!(
            ext.to_lowercase().as_str(),
            "txt"
                | "text"
                | "log"
                | "sql"
                | "sh"
                | "bash"
                | "ps1"
                | "bat"
                | "cmd"
                | "env"
                | "ini"
                | "cfg"
                | "conf"
                | "properties"
                | "html"
                | "htm"
        )
    }

    /// Checks whether an exact filename (case-insensitive) or dot-file without normal extension is supported.
    pub fn supports_filename(&self, file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        matches!(lower.as_str(), "dockerfile" | "makefile")
            || lower.ends_with(".dockerfile")
            || lower.ends_with(".makefile")
            || lower == ".env"
            || lower.starts_with(".env.")
            || lower.ends_with(".env")
    }

    /// Checks whether the target is a Web URL.
    pub fn is_web_url(&self, target: &str) -> bool {
        target.starts_with("http://") || target.starts_with("https://")
    }

    /// Resolves the specific semantic content type for a given path or URL.
    pub fn resolve_content_type(&self, file_path: &str) -> &'static str {
        if self.is_web_url(file_path) {
            return "web";
        }

        let p = Path::new(file_path);
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();

        if file_name == "dockerfile" || file_name.ends_with(".dockerfile")
            || file_name == "makefile" || file_name.ends_with(".makefile")
        {
            return "build";
        }

        if file_name == ".env" || file_name.starts_with(".env.") || file_name.ends_with(".env") {
            return "config";
        }

        match ext.as_str() {
            "sql" => "sql",
            "sh" | "bash" | "ps1" | "bat" | "cmd" => "shell",
            "env" | "ini" | "cfg" | "conf" | "properties" => "config",
            "html" | "htm" => "web",
            _ => "text",
        }
    }
}

impl Chunker for TextChunker {
    fn chunk(&self, file_path: &str, content: &str) -> Result<Vec<ChunkPayload>, String> {
        let clean = content.trim();
        if clean.is_empty() {
            return Ok(Vec::new());
        }

        let is_web = self.is_web_url(file_path);
        let p = Path::new(file_path);
        let file_name = if is_web {
            file_path.to_string()
        } else {
            p.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("document.txt")
                .to_string()
        };

        let content_type = self.resolve_content_type(file_path);
        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Ok(Vec::new());
        }

        let mut chunks: Vec<ChunkPayload> = Vec::new();
        let mut current_lines: Vec<&str> = Vec::new();
        let mut current_chars = 0usize;
        let mut start_line = 1usize;
        let mut current_line_idx = 1usize;

        for (idx, line) in lines.iter().enumerate() {
            let line_len = line.len() + 1; // including \n

            // If adding this line exceeds max_chunk_chars and we already have content
            if current_chars + line_len > self.max_chunk_chars && !current_lines.is_empty() {
                let end_line = start_line + current_lines.len().saturating_sub(1);
                let chunk_text_raw = current_lines.join("\n");
                let label = if is_web {
                    format!("// Context: {} > Section (lines {}..{})\n---\n{}", file_path, start_line, end_line, chunk_text_raw)
                } else {
                    format!("// Context: {} > lines {}..{}\n---\n{}", file_name, start_line, end_line, chunk_text_raw)
                };

                let header_path = if is_web {
                    format!("{} > lines {}..{}", file_path, start_line, end_line)
                } else {
                    format!("{} > lines {}..{}", file_name, start_line, end_line)
                };

                chunks.push(ChunkPayload::new(
                    format!("{}_{}", file_name.replace(['/', '\\', ':', '.'], "_"), chunks.len()),
                    label,
                    file_name.clone(),
                    file_path.to_string(),
                    Some(header_path),
                    start_line,
                    end_line,
                    content_type.to_string(),
                    chunks.len(),
                ));

                // Calculate overlap lines
                let mut overlap_acc = 0usize;
                let mut overlap_slice_start = current_lines.len();
                if self.overlap_chars > 0 {
                    for i in (0..current_lines.len()).rev() {
                        overlap_acc += current_lines[i].len() + 1;
                        overlap_slice_start = i;
                        if overlap_acc >= self.overlap_chars {
                            break;
                        }
                    }
                }

                if overlap_slice_start < current_lines.len() {
                    let kept_lines: Vec<&str> = current_lines[overlap_slice_start..].to_vec();
                    start_line = start_line + overlap_slice_start;
                    current_chars = kept_lines.iter().map(|l| l.len() + 1).sum();
                    current_lines = kept_lines;
                } else {
                    current_lines.clear();
                    current_chars = 0;
                    start_line = idx + 1;
                }
            }

            current_lines.push(line);
            current_chars += line_len;
            current_line_idx = idx + 1;
        }

        // Flush remaining lines
        if !current_lines.is_empty() {
            let end_line = current_line_idx;
            let chunk_text_raw = current_lines.join("\n");
            let label = if is_web {
                format!("// Context: {} > Section (lines {}..{})\n---\n{}", file_path, start_line, end_line, chunk_text_raw)
            } else {
                format!("// Context: {} > lines {}..{}\n---\n{}", file_name, start_line, end_line, chunk_text_raw)
            };

            let header_path = if is_web {
                format!("{} > lines {}..{}", file_path, start_line, end_line)
            } else {
                format!("{} > lines {}..{}", file_name, start_line, end_line)
            };

            chunks.push(ChunkPayload::new(
                format!("{}_{}", file_name.replace(['/', '\\', ':', '.'], "_"), chunks.len()),
                label,
                file_name,
                file_path.to_string(),
                Some(header_path),
                start_line,
                end_line,
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
    fn test_text_chunker_supports() {
        let chunker = TextChunker::default();
        assert!(chunker.supports_extension("txt"));
        assert!(chunker.supports_extension("log"));
        assert!(chunker.supports_extension("sql"));
        assert!(chunker.supports_extension("sh"));
        assert!(chunker.supports_extension("bash"));
        assert!(chunker.supports_extension("ps1"));
        assert!(chunker.supports_extension("bat"));
        assert!(chunker.supports_extension("env"));
        assert!(chunker.supports_extension("ini"));
        assert!(chunker.supports_extension("properties"));
        assert!(chunker.supports_filename("Dockerfile"));
        assert!(chunker.supports_filename("Makefile"));
        assert!(chunker.is_web_url("https://example.com/docs"));
        assert!(chunker.is_web_url("http://localhost:8000"));

        assert_eq!(chunker.resolve_content_type("script.sql"), "sql");
        assert_eq!(chunker.resolve_content_type("deploy.sh"), "shell");
        assert_eq!(chunker.resolve_content_type(".env"), "config");
        assert_eq!(chunker.resolve_content_type("Dockerfile"), "build");
        assert_eq!(chunker.resolve_content_type("https://example.com"), "web");
        assert_eq!(chunker.resolve_content_type("notes.txt"), "text");
    }

    #[test]
    fn test_text_chunker_basic() {
        let chunker = TextChunker::new(50, 10);
        let content = "SELECT * FROM users;\nSELECT * FROM orders;\nSELECT * FROM items;\nSELECT * FROM payments;\n";
        let chunks = chunker.chunk("schema.sql", content).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "sql");
        assert!(chunks[0].text.contains("// Context: schema.sql > lines"));
    }

    #[test]
    fn test_web_url_chunking() {
        let chunker = TextChunker::new(100, 20);
        let content = "Welcome to our documentation.\nHere is how to get started with our API.\nCall /v1/health to verify.\n";
        let chunks = chunker.chunk("https://api.example.com/docs", content).expect("Chunking failed");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "web");
        assert!(chunks[0].text.contains("// Context: https://api.example.com/docs > Section (lines"));
    }
}
