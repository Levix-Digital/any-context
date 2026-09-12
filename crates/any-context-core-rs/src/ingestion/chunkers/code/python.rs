use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct PythonASTParser;

impl PythonASTParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for PythonASTParser {
    fn parse_chunks(
        &self,
        file_path: &str,
        content: &str,
        max_chunk_chars: usize,
    ) -> Result<Vec<ChunkPayload>, String> {
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let mut parser = Parser::new();
        let language: Language = tree_sitter_python::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load Python grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse Python code".to_string())?;

        let root_node = tree.root_node();
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(file_path);

        let mut chunks = Vec::new();
        let mut last_covered_byte = 0;

        let child_count = root_node.child_count();
        for i in 0..child_count {
            let node = root_node.child(i).unwrap();
            let kind = node.kind();

            match kind {
                "function_definition" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        node.start_byte(),
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_function(
                        file_path,
                        file_name,
                        content,
                        node,
                        None,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = node.end_byte();
                }
                "class_definition" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        node.start_byte(),
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_class(
                        file_path,
                        file_name,
                        content,
                        node,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = node.end_byte();
                }
                "decorated_definition" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        node.start_byte(),
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_decorated(
                        file_path,
                        file_name,
                        content,
                        node,
                        None,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = node.end_byte();
                }
                _ => {
                    // Statements outside functions/classes (imports, constants, top-level code)
                    // These will be captured either by capture_preceding_gap or at end of file
                }
            }
        }

        // Capture remaining trailing code (e.g. if __name__ == "__main__": etc.)
        if last_covered_byte < content.len() {
            self.capture_preceding_gap(
                file_path,
                file_name,
                content,
                last_covered_byte,
                content.len(),
                &mut chunks,
                max_chunk_chars,
            );
        }

        Ok(chunks)
    }
}

impl PythonASTParser {
    fn extract_name(&self, node: Node, source: &str) -> Option<String> {
        node.child_by_field_name("name")
            .map(|name_node| source[name_node.start_byte()..name_node.end_byte()].to_string())
    }

    fn process_function(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        class_scope: Option<&str>,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let fn_name = self
            .extract_name(node, content)
            .unwrap_or_else(|| "anonymous_function".to_string());

        let (header_path, breadcrumb_label) = match class_scope {
            Some(cls) => (
                format!("class {} > def {}", cls, fn_name),
                format!("class {} > def {}", cls, fn_name),
            ),
            None => (format!("def {}", fn_name), format!("def {}", fn_name)),
        };

        let raw_code = &content[node.start_byte()..node.end_byte()];
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &header_path,
            &breadcrumb_label,
            raw_code,
            node.start_byte(),
            node.end_byte(),
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }

    fn process_decorated(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        class_scope: Option<&str>,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        // Find inner function or class definition
        let mut inner_def = None;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "function_definition" || child.kind() == "class_definition" {
                    inner_def = Some(child);
                    break;
                }
            }
        }

        let Some(inner) = inner_def else {
            return;
        };

        if inner.kind() == "class_definition" {
            // Decorated class
            self.process_class(
                file_path,
                file_name,
                content,
                node, // use outer decorated node for byte range
                chunks,
                max_chunk_chars,
            );
        } else {
            // Decorated function / method
            let fn_name = self
                .extract_name(inner, content)
                .unwrap_or_else(|| "anonymous_function".to_string());

            let (header_path, breadcrumb_label) = match class_scope {
                Some(cls) => (
                    format!("class {} > def {}", cls, fn_name),
                    format!("class {} > def {}", cls, fn_name),
                ),
                None => (format!("def {}", fn_name), format!("def {}", fn_name)),
            };

            let raw_code = &content[node.start_byte()..node.end_byte()];
            let start_line = node.start_position().row + 1;
            let end_line = node.end_position().row + 1;

            self.emit_or_split_code_chunk(
                file_path,
                file_name,
                &header_path,
                &breadcrumb_label,
                raw_code,
                node.start_byte(),
                node.end_byte(),
                start_line,
                end_line,
                chunks,
                max_chunk_chars,
            );
        }
    }

    fn process_class(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        // If node is decorated_definition, inner is class_definition
        let class_node = if node.kind() == "decorated_definition" {
            let mut found = node;
            for i in 0..node.child_count() {
                if let Some(c) = node.child(i) {
                    if c.kind() == "class_definition" {
                        found = c;
                        break;
                    }
                }
            }
            found
        } else {
            node
        };

        let class_name = self
            .extract_name(class_node, content)
            .unwrap_or_else(|| "AnonymousClass".to_string());

        let raw_code = &content[node.start_byte()..node.end_byte()];
        let start_line = node.start_position().row + 1;
        let end_line = node.end_position().row + 1;

        // If the entire class fits within max_chunk_chars, keep it whole and cohesive
        if raw_code.len() <= max_chunk_chars {
            let header_path = format!("class {}", class_name);
            let breadcrumb = format!("class {}", class_name);
            self.emit_single_chunk(
                file_path,
                file_name,
                &header_path,
                &breadcrumb,
                raw_code,
                node.start_byte(),
                node.end_byte(),
                start_line,
                end_line,
                chunks,
            );
            return;
        }

        // Class exceeds max_chunk_chars: split methods and overview
        let Some(body_node) = class_node.child_by_field_name("body") else {
            let header_path = format!("class {}", class_name);
            self.emit_single_chunk(
                file_path,
                file_name,
                &header_path,
                &header_path,
                raw_code,
                node.start_byte(),
                node.end_byte(),
                start_line,
                end_line,
                chunks,
            );
            return;
        };

        // Find first method inside class body
        let mut first_method_byte = None;
        for i in 0..body_node.child_count() {
            let child = body_node.child(i).unwrap();
            if child.kind() == "function_definition" || child.kind() == "decorated_definition" {
                first_method_byte = Some(child.start_byte());
                break;
            }
        }

        // Emit Class Overview / Attributes chunk (header before methods)
        let overview_end_byte = first_method_byte.unwrap_or(node.end_byte());
        let overview_code = content[node.start_byte()..overview_end_byte].trim();
        if !overview_code.is_empty() {
            let header_path = format!("class {} (Overview)", class_name);
            let breadcrumb = format!("class {} [Overview]", class_name);
            let overview_end_line =
                content[..overview_end_byte].lines().count().max(start_line);

            self.emit_single_chunk(
                file_path,
                file_name,
                &header_path,
                &breadcrumb,
                overview_code,
                node.start_byte(),
                overview_end_byte,
                start_line,
                overview_end_line,
                chunks,
            );
        }

        // Emit each method inside class
        for i in 0..body_node.child_count() {
            let child = body_node.child(i).unwrap();
            match child.kind() {
                "function_definition" => {
                    self.process_function(
                        file_path,
                        file_name,
                        content,
                        child,
                        Some(&class_name),
                        chunks,
                        max_chunk_chars,
                    );
                }
                "decorated_definition" => {
                    self.process_decorated(
                        file_path,
                        file_name,
                        content,
                        child,
                        Some(&class_name),
                        chunks,
                        max_chunk_chars,
                    );
                }
                _ => {
                    // Comments or secondary statements inside class
                }
            }
        }
    }

    fn capture_preceding_gap(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        start_byte: usize,
        end_byte: usize,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        if start_byte >= end_byte || start_byte >= content.len() {
            return;
        }

        let slice = &content[start_byte..end_byte];
        let trimmed = slice.trim();
        if trimmed.is_empty() {
            return;
        }

        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);
        let header_path = "Module Overview / Statements".to_string();
        let breadcrumb = "Module Overview / Statements".to_string();

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &header_path,
            &breadcrumb,
            trimmed,
            start_byte,
            end_byte,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }

    fn emit_single_chunk(
        &self,
        file_path: &str,
        file_name: &str,
        header_path: &str,
        breadcrumb_label: &str,
        code: &str,
        _start_char: usize,
        _end_char: usize,
        start_line: usize,
        end_line: usize,
        chunks: &mut Vec<ChunkPayload>,
    ) {
        let chunk_text = format!(
            "// Context: {} > {} [lines {}-{}]\n---\n{}",
            file_name, breadcrumb_label, start_line, end_line, code
        );

        let chunk_id = format!(
            "{:x}",
            md5_hash(format!("{}::{}:{}", file_path, start_line, header_path).as_bytes())
        );

        let chunk_index = chunks.len();
        chunks.push(ChunkPayload {
            id: chunk_id,
            text: chunk_text,
            file_name: file_name.to_string(),
            file_path: file_path.to_string(),
            header_path: Some(header_path.to_string()),
            start_line,
            end_line,
            content_type: "python".to_string(),
            chunk_index,
        });
    }

    fn emit_or_split_code_chunk(
        &self,
        file_path: &str,
        file_name: &str,
        header_path: &str,
        breadcrumb_label: &str,
        code: &str,
        start_char: usize,
        end_char: usize,
        start_line: usize,
        end_line: usize,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let parts = crate::ingestion::chunkers::code::splitter::split_oversized_code(code, max_chunk_chars);
        let total_parts = parts.len();

        for (idx, part) in parts.into_iter().enumerate() {
            let (part_breadcrumb, part_header) = if total_parts > 1 {
                (
                    format!("{} (Part {})", breadcrumb_label, idx + 1),
                    format!("{} (Part {})", header_path, idx + 1),
                )
            } else {
                (breadcrumb_label.to_string(), header_path.to_string())
            };

            let part_start = start_line + part.start_line_offset;
            let part_end = (start_line + part.end_line_offset).min(end_line).max(part_start);

            self.emit_single_chunk(
                file_path,
                file_name,
                &part_header,
                &part_breadcrumb,
                &part.text,
                start_char,
                end_char,
                part_start,
                part_end,
                chunks,
            );
        }
    }
}

/// Simple fast hashing helper
fn md5_hash(data: &[u8]) -> u128 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut hasher = DefaultHasher::new();
    hasher.write(data);
    hasher.finish() as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_function_chunking() {
        let code = r#"
import os

def calculate_hash(data: str) -> str:
    """Calculate SHA-256 hash."""
    import hashlib
    return hashlib.sha256(data.encode()).hexdigest()

def verify_hash(data: str, expected: str) -> bool:
    return calculate_hash(data) == expected
"#;

        let parser = PythonASTParser::new();
        let chunks = parser.parse_chunks("src/crypto.py", code, 1800).unwrap();

        assert!(chunks.len() >= 2);
        let fn_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.header_path.as_deref().unwrap_or("").starts_with("def "))
            .collect();
        assert_eq!(fn_chunks.len(), 2);
        assert_eq!(fn_chunks[0].header_path.as_deref(), Some("def calculate_hash"));
        assert!(fn_chunks[0].text.contains("// Context: crypto.py > def calculate_hash"));
        assert!(fn_chunks[0].text.contains("Calculate SHA-256 hash"));
        assert_eq!(fn_chunks[1].header_path.as_deref(), Some("def verify_hash"));
    }

    #[test]
    fn test_python_class_with_methods() {
        let code = r#"
class PaymentGateway:
    """Enterprise payment gateway client."""
    timeout: int = 30

    @classmethod
    def create_default(cls):
        return cls()

    def process_charge(self, amount: int) -> bool:
        if amount <= 0:
            return False
        return True
"#;

        let parser = PythonASTParser::new();
        // Small max_chunk_chars to force class method splitting
        let chunks = parser.parse_chunks("src/payments.py", code, 120).unwrap();

        // Should have overview and methods
        let overview = chunks
            .iter()
            .find(|c| c.header_path.as_deref() == Some("class PaymentGateway (Overview)"));
        assert!(overview.is_some());
        assert!(overview.unwrap().text.contains("Enterprise payment gateway client"));

        let method1 = chunks
            .iter()
            .find(|c| c.header_path.as_deref() == Some("class PaymentGateway > def create_default"));
        assert!(method1.is_some());
        assert!(method1.unwrap().text.contains("@classmethod"));

        let method2 = chunks
            .iter()
            .find(|c| c.header_path.as_deref() == Some("class PaymentGateway > def process_charge"));
        assert!(method2.is_some());
        assert!(method2.unwrap().text.contains("def process_charge"));
    }

    #[test]
    fn test_empty_python_file() {
        let parser = PythonASTParser::new();
        let chunks = parser.parse_chunks("empty.py", "  \n\n\t  ", 1800).unwrap();
        assert!(chunks.is_empty());
    }
}
