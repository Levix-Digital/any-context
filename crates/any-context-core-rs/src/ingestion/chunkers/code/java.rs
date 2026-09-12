use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct JavaASTParser;

impl JavaASTParser {
    pub fn new() -> Self {
        Self
    }

    fn extract_identifier(node: Node, content: &str) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(content[n.start_byte()..n.end_byte()].to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind() == "identifier" {
                return Some(content[child.start_byte()..child.end_byte()].to_string());
            }
        }
        None
    }

    fn find_preceding_doc_comment<'a>(&self, node: Node, content: &'a str) -> Option<(usize, &'a str)> {
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            let kind = p.kind();
            if kind == "block_comment" || kind == "line_comment" {
                let c_text = &content[p.start_byte()..p.end_byte()];
                if c_text.starts_with("/**") || c_text.starts_with("//") {
                    return Some((p.start_byte(), c_text));
                }
            }
            if !p.is_extra() && kind != "block_comment" && kind != "line_comment" {
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
            content_type: "java".to_string(),
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
            chunks,
            max_chunk_chars,
        );
    }

    fn process_method(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        class_name: &str,
        kind_label: &str,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let method_name = Self::extract_identifier(node, content).unwrap_or_else(|| "unnamed".to_string());
        let mut start_byte = node.start_byte();
        let end_byte = node.end_byte();

        if let Some((doc_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_byte = doc_start;
        }

        let method_text = content[start_byte..end_byte].trim();
        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);

        let symbol_desc = format!("class {} > {} {}", class_name, kind_label, method_name);
        let header_path = format!("class {} > {} {}", class_name, kind_label, method_name);

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &header_path,
            &symbol_desc,
            method_text,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }

    fn process_class_or_type(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        kind_label: &str,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let name = Self::extract_identifier(node, content).unwrap_or_else(|| "Anonymous".to_string());
        let mut start_byte = node.start_byte();
        let end_byte = node.end_byte();

        if let Some((doc_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_byte = doc_start;
        }

        let item_text = content[start_byte..end_byte].trim();
        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);

        let header_path = format!("{} {}", kind_label, name);
        let breadcrumb_label = format!("{} {}", kind_label, name);

        if item_text.len() <= max_chunk_chars {
            self.emit_single_chunk(
                file_path,
                file_name,
                &header_path,
                &breadcrumb_label,
                item_text,
                start_line,
                end_line,
                chunks,
            );
            return;
        }

        // Oversized: create Overview + split inner methods
        let body_node = (0..node.child_count())
            .map(|i| node.child(i).unwrap())
            .find(|c| c.kind() == "class_body" || c.kind() == "interface_body" || c.kind() == "record_declaration_body");

        if let Some(body) = body_node {
            let header_end_byte = body.start_byte().min(end_byte);
            let header_text = content[start_byte..header_end_byte].trim();
            let ov_header = format!("{} {} (Overview)", kind_label, name);
            let ov_crumb = format!("{} {} [Overview]", kind_label, name);
            let ov_end_line = start_line + header_text.lines().count().saturating_sub(1);

            self.emit_single_chunk(
                file_path,
                file_name,
                &ov_header,
                &ov_crumb,
                &format!("{}\n  // ... (methods split into dedicated chunks)", header_text),
                start_line,
                ov_end_line,
                chunks,
            );

            for i in 0..body.child_count() {
                let member = body.child(i).unwrap();
                match member.kind() {
                    "method_declaration" => {
                        self.process_method(
                            file_path,
                            file_name,
                            content,
                            member,
                            &name,
                            "method",
                            chunks,
                            max_chunk_chars,
                        );
                    }
                    "constructor_declaration" => {
                        self.process_method(
                            file_path,
                            file_name,
                            content,
                            member,
                            &name,
                            "constructor",
                            chunks,
                            max_chunk_chars,
                        );
                    }
                    _ => {}
                }
            }
        } else {
            self.emit_or_split_code_chunk(
                file_path,
                file_name,
                &header_path,
                &breadcrumb_label,
                item_text,
                start_line,
                end_line,
                chunks,
                max_chunk_chars,
            );
        }
    }
}

impl Default for JavaASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for JavaASTParser {
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
        let language: Language = tree_sitter_java::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load Java grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse Java code".to_string())?;

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
                "class_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        node.start_byte(),
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_class_or_type(
                        file_path,
                        file_name,
                        content,
                        node,
                        "class",
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = node.end_byte();
                }
                "interface_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        node.start_byte(),
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_class_or_type(
                        file_path,
                        file_name,
                        content,
                        node,
                        "interface",
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = node.end_byte();
                }
                "record_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        node.start_byte(),
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_class_or_type(
                        file_path,
                        file_name,
                        content,
                        node,
                        "record",
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = node.end_byte();
                }
                "enum_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        node.start_byte(),
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_class_or_type(
                        file_path,
                        file_name,
                        content,
                        node,
                        "enum",
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = node.end_byte();
                }
                _ => {}
            }
        }

        // Capture trailing gap
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
    fn test_java_class_and_methods() {
        let code = r#"
package com.example.service;

import java.util.List;

/**
 * Service for managing user accounts.
 */
public class UserService {
    private final UserRepository repository;

    public UserService(UserRepository repository) {
        this.repository = repository;
    }

    public User findById(Long id) {
        return repository.findById(id).orElse(null);
    }
}
"#;

        let parser = JavaASTParser::new();
        let chunks = parser.parse_chunks("src/UserService.java", code, 1800).unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("class UserService")));
    }
}
