use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct CppASTParser;

impl CppASTParser {
    pub fn new() -> Self {
        Self
    }

    fn select_language(file_path: &str) -> (Language, &'static str) {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "c" => (tree_sitter_c::LANGUAGE.into(), "c"),
            "h" => (tree_sitter_cpp::LANGUAGE.into(), "c"),
            _ => (tree_sitter_cpp::LANGUAGE.into(), "cpp"),
        }
    }

    fn find_name_in_declarator(node: Node, content: &str) -> Option<String> {
        match node.kind() {
            "identifier"
            | "field_identifier"
            | "type_identifier"
            | "qualified_identifier"
            | "destructor_name"
            | "operator_name" => Some(content[node.start_byte()..node.end_byte()].to_string()),
            _ => {
                if let Some(inner) = node.child_by_field_name("declarator") {
                    if let Some(res) = Self::find_name_in_declarator(inner, content) {
                        return Some(res);
                    }
                }
                for i in 0..node.child_count() {
                    let child = node.child(i).unwrap();
                    if let Some(res) = Self::find_name_in_declarator(child, content) {
                        return Some(res);
                    }
                }
                None
            }
        }
    }

    fn extract_function_name(node: Node, content: &str) -> Option<String> {
        if let Some(decl) = node.child_by_field_name("declarator") {
            Self::find_name_in_declarator(decl, content)
        } else {
            None
        }
    }

    fn extract_type_name(node: Node, content: &str) -> Option<String> {
        if let Some(name_node) = node.child_by_field_name("name") {
            return Some(content[name_node.start_byte()..name_node.end_byte()].to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind() == "type_identifier" || child.kind() == "identifier" {
                return Some(content[child.start_byte()..child.end_byte()].to_string());
            }
        }
        None
    }

    fn find_preceding_doc_comment<'a>(&self, node: Node, content: &'a str) -> Option<(usize, &'a str)> {
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            let kind = p.kind();
            if kind == "comment" {
                let c_text = &content[p.start_byte()..p.end_byte()];
                if c_text.starts_with("//") || c_text.starts_with("/*") {
                    return Some((p.start_byte(), c_text));
                }
            }
            if !p.is_extra() && kind != "comment" {
                break;
            }
            prev = p.prev_sibling();
        }
        None
    }

    fn emit_single_chunk(
        &self,
        file_path: &str,
        file_name: &str,
        header_path: &str,
        breadcrumb_label: &str,
        code: &str,
        start_line: usize,
        end_line: usize,
        content_type: &str,
        chunks: &mut Vec<ChunkPayload>,
    ) {
        let chunk_text = format!(
            "// Context: {} > {} [lines {}-{}]\n---\n{}",
            file_name, breadcrumb_label, start_line, end_line, code
        );

        let mut hasher = DefaultHasher::new();
        hasher.write(format!("{}::{}:{}", file_path, start_line, header_path).as_bytes());
        let chunk_id = format!("{:x}", hasher.finish() as u128);

        let chunk_index = chunks.len();
        chunks.push(ChunkPayload {
            id: chunk_id,
            text: chunk_text,
            file_name: file_name.to_string(),
            file_path: file_path.to_string(),
            header_path: Some(header_path.to_string()),
            start_line,
            end_line,
            content_type: content_type.to_string(),
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
        start_line: usize,
        end_line: usize,
        content_type: &str,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        if code.len() <= max_chunk_chars {
            self.emit_single_chunk(
                file_path,
                file_name,
                header_path,
                breadcrumb_label,
                code,
                start_line,
                end_line,
                content_type,
                chunks,
            );
            return;
        }

        let paragraphs: Vec<&str> = code.split("\n\n").collect();
        let mut current_block = String::new();
        let mut part_idx = 1;
        let mut current_start_line = start_line;

        for p in paragraphs {
            if current_block.len() + p.len() + 2 > max_chunk_chars && !current_block.is_empty() {
                let current_end_line =
                    current_start_line + current_block.lines().count().saturating_sub(1);
                let part_breadcrumb = format!("{} (Part {})", breadcrumb_label, part_idx);
                let part_header = format!("{} (Part {})", header_path, part_idx);

                self.emit_single_chunk(
                    file_path,
                    file_name,
                    &part_header,
                    &part_breadcrumb,
                    current_block.trim(),
                    current_start_line,
                    current_end_line,
                    content_type,
                    chunks,
                );

                current_start_line = current_end_line + 1;
                part_idx += 1;
                current_block.clear();
            }

            if !current_block.is_empty() {
                current_block.push_str("\n\n");
            }
            current_block.push_str(p);
        }

        if !current_block.is_empty() {
            let part_breadcrumb = if part_idx > 1 {
                format!("{} (Part {})", breadcrumb_label, part_idx)
            } else {
                breadcrumb_label.to_string()
            };
            let part_header = if part_idx > 1 {
                format!("{} (Part {})", header_path, part_idx)
            } else {
                header_path.to_string()
            };

            self.emit_single_chunk(
                file_path,
                file_name,
                &part_header,
                &part_breadcrumb,
                current_block.trim(),
                current_start_line,
                end_line,
                content_type,
                chunks,
            );
        }
    }

    fn capture_preceding_gap(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        start_byte: usize,
        end_byte: usize,
        content_type: &str,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        if end_byte <= start_byte {
            return;
        }

        let gap_text = &content[start_byte..end_byte];
        let trimmed = gap_text.trim();
        if trimmed.is_empty() {
            return;
        }

        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            "module",
            "module",
            trimmed,
            start_line,
            end_line,
            content_type,
            chunks,
            max_chunk_chars,
        );
    }

    fn collect_declarations<'a>(
        &self,
        parent: Node<'a>,
        scope_prefix: &str,
        content: &str,
        declarations: &mut Vec<(Node<'a>, String)>,
    ) {
        let count = parent.child_count();
        for i in 0..count {
            let child = parent.child(i).unwrap();
            let kind = child.kind();

            match kind {
                "function_definition"
                | "class_specifier"
                | "struct_specifier"
                | "enum_specifier"
                | "template_declaration" => {
                    declarations.push((child, scope_prefix.to_string()));
                }
                "namespace_definition" => {
                    let ns_name = child
                        .child_by_field_name("name")
                        .map(|n| &content[n.start_byte()..n.end_byte()])
                        .unwrap_or("");
                    let new_scope = if scope_prefix.is_empty() {
                        format!("namespace {}", ns_name)
                    } else {
                        format!("{} > namespace {}", scope_prefix, ns_name)
                    };

                    let body = child.child_by_field_name("body");
                    if let Some(b) = body {
                        self.collect_declarations(b, &new_scope, content, declarations);
                    } else {
                        self.collect_declarations(child, &new_scope, content, declarations);
                    }
                }
                "declaration" => {
                    // Check if declaration wraps a class_specifier, struct_specifier, or enum_specifier
                    for j in 0..child.child_count() {
                        let inner = child.child(j).unwrap();
                        match inner.kind() {
                            "class_specifier" | "struct_specifier" | "enum_specifier" => {
                                declarations.push((child, scope_prefix.to_string()));
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for CppASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for CppASTParser {
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

        let (language, content_type) = Self::select_language(file_path);
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load C/C++ grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse C/C++ code".to_string())?;

        let root_node = tree.root_node();
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(file_path);

        let mut chunks = Vec::new();
        let mut last_covered_byte = 0;

        let mut declarations = Vec::new();
        self.collect_declarations(root_node, "", content, &mut declarations);

        for (node, scope_prefix) in declarations {
            let mut start_byte = node.start_byte();
            let mut end_byte = node.end_byte();

            // If immediately followed by a semicolon sibling (common with class/struct specifiers), swallow it
            if let Some(next) = node.next_sibling() {
                if next.kind() == ";" {
                    end_byte = next.end_byte();
                }
            }

            if let Some((doc_start, _)) = self.find_preceding_doc_comment(node, content) {
                start_byte = doc_start;
            }

            self.capture_preceding_gap(
                file_path,
                file_name,
                content,
                last_covered_byte,
                start_byte,
                content_type,
                &mut chunks,
                max_chunk_chars,
            );

            let kind = node.kind();
            let (kind_label, symbol_name) = match kind {
                "function_definition" => {
                    let name = Self::extract_function_name(node, content)
                        .unwrap_or_else(|| "unnamed".to_string());
                    ("function", name)
                }
                "class_specifier" => {
                    let name = Self::extract_type_name(node, content)
                        .unwrap_or_else(|| "Anonymous".to_string());
                    ("class", name)
                }
                "struct_specifier" => {
                    let name = Self::extract_type_name(node, content)
                        .unwrap_or_else(|| "Anonymous".to_string());
                    ("struct", name)
                }
                "enum_specifier" => {
                    let name = Self::extract_type_name(node, content)
                        .unwrap_or_else(|| "Anonymous".to_string());
                    ("enum", name)
                }
                "template_declaration" => {
                    // Check if child is class or function
                    let mut label = "template";
                    let mut name = "unnamed".to_string();
                    for j in 0..node.child_count() {
                        let c = node.child(j).unwrap();
                        match c.kind() {
                            "class_specifier" => {
                                label = "template class";
                                name = Self::extract_type_name(c, content)
                                    .unwrap_or_else(|| "Anonymous".to_string());
                                break;
                            }
                            "struct_specifier" => {
                                label = "template struct";
                                name = Self::extract_type_name(c, content)
                                    .unwrap_or_else(|| "Anonymous".to_string());
                                break;
                            }
                            "function_definition" => {
                                label = "template function";
                                name = Self::extract_function_name(c, content)
                                    .unwrap_or_else(|| "unnamed".to_string());
                                break;
                            }
                            _ => {}
                        }
                    }
                    (label, name)
                }
                "declaration" => {
                    // Look for inner class/struct/enum specifier
                    let mut l = "declaration";
                    let mut n = "anonymous".to_string();
                    for j in 0..node.child_count() {
                        let c = node.child(j).unwrap();
                        match c.kind() {
                            "class_specifier" => {
                                l = "class";
                                n = Self::extract_type_name(c, content)
                                    .unwrap_or_else(|| "Anonymous".to_string());
                                break;
                            }
                            "struct_specifier" => {
                                l = "struct";
                                n = Self::extract_type_name(c, content)
                                    .unwrap_or_else(|| "Anonymous".to_string());
                                break;
                            }
                            "enum_specifier" => {
                                l = "enum";
                                n = Self::extract_type_name(c, content)
                                    .unwrap_or_else(|| "Anonymous".to_string());
                                break;
                            }
                            _ => {}
                        }
                    }
                    (l, n)
                }
                _ => ("code", "item".to_string()),
            };

            let item_text = content[start_byte..end_byte].trim();
            let start_line = content[..start_byte].lines().count().max(1);
            let end_line = content[..end_byte].lines().count().max(start_line);

            let label = if scope_prefix.is_empty() {
                format!("{} {}", kind_label, symbol_name)
            } else {
                format!("{} > {} {}", scope_prefix, kind_label, symbol_name)
            };

            self.emit_or_split_code_chunk(
                file_path,
                file_name,
                &label,
                &label,
                item_text,
                start_line,
                end_line,
                content_type,
                &mut chunks,
                max_chunk_chars,
            );

            last_covered_byte = end_byte;
        }

        // Capture trailing gap
        if last_covered_byte < content.len() {
            self.capture_preceding_gap(
                file_path,
                file_name,
                content,
                last_covered_byte,
                content.len(),
                content_type,
                &mut chunks,
                max_chunk_chars,
            );
        }

        if chunks.is_empty() && !trimmed.is_empty() {
            let start_line = 1;
            let end_line = content.lines().count().max(1);
            self.emit_single_chunk(
                file_path,
                file_name,
                "module",
                "module",
                trimmed,
                start_line,
                end_line,
                content_type,
                &mut chunks,
            );
        }

        for (idx, chunk) in chunks.iter_mut().enumerate() {
            chunk.chunk_index = idx;
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_classes_namespaces_and_functions() {
        let code = r#"
#include <iostream>
#include <vector>

namespace core {
    class Worker {
    public:
        void run();
    };

    struct Config {
        int timeout_ms;
    };

    void Worker::run() {
        std::cout << "Running worker..." << std::endl;
    }
}

template<typename T>
class RingBuffer {
    std::vector<T> items;
};

int main(int argc, char** argv) {
    core::Worker worker;
    worker.run();
    return 0;
}
"#;

        let parser = CppASTParser::new();
        let chunks = parser.parse_chunks("src/main.cpp", code, 1800).unwrap();
        assert!(!chunks.is_empty());

        // Check module gap
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("module") && c.text.contains("#include <iostream>")));

        // Check namespace > class
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("namespace core > class Worker")));

        // Check namespace > struct
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("namespace core > struct Config")));

        // Check namespace > method definition
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("namespace core > function Worker::run")));

        // Check template class
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("template class RingBuffer")));

        // Check standalone function main
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("function main")));
    }

    #[test]
    fn test_c_functions_and_structs() {
        let code = r#"
#include <stdio.h>
#include <stdlib.h>

struct Packet {
    int id;
    int length;
};

int process_packet(struct Packet* pkt) {
    if (pkt == NULL) return -1;
    return pkt->length;
}
"#;

        let parser = CppASTParser::new();
        let chunks = parser.parse_chunks("src/packet.c", code, 1800).unwrap();
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].content_type, "c");
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("struct Packet")));
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("function process_packet")));
    }
}
