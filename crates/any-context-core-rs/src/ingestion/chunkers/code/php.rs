use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct PhpASTParser;

impl PhpASTParser {
    pub fn new() -> Self {
        Self
    }

    fn extract_identifier(node: Node, content: &str) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(content[n.start_byte()..n.end_byte()].to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            let k = child.kind();
            if k == "name" || k == "identifier" {
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
                if c_text.starts_with("/**") || c_text.starts_with("/*") || c_text.starts_with("//") || c_text.starts_with('#') {
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
            content_type: "php".to_string(),
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
                part_start,
                part_end,
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
            "Namespace & Imports",
            "namespace/imports",
            trimmed,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }
}

impl Default for PhpASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for PhpASTParser {
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
        let language: Language = tree_sitter_php::LANGUAGE_PHP.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load PHP grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse PHP code".to_string())?;

        let root_node = tree.root_node();
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(file_path);

        let mut chunks = Vec::new();
        let mut last_covered_byte = 0;

        let child_count = root_node.child_count();
        for i in 0..child_count {
            let child = root_node.child(i).unwrap();
            let kind = child.kind();

            match kind {
                "class_declaration" | "interface_declaration" | "trait_declaration" | "enum_declaration" => {
                    let mut start_b = child.start_byte();
                    if let Some((c_start, _)) = self.find_preceding_doc_comment(child, content) {
                        start_b = c_start;
                    }

                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        start_b,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    self.process_type(
                        file_path,
                        file_name,
                        content,
                        child,
                        None,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = child.end_byte();
                }
                "function_definition" => {
                    let mut start_b = child.start_byte();
                    if let Some((c_start, _)) = self.find_preceding_doc_comment(child, content) {
                        start_b = c_start;
                    }

                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        start_b,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    self.process_function(
                        file_path,
                        file_name,
                        content,
                        child,
                        None,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = child.end_byte();
                }
                _ => {}
            }
        }

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

impl PhpASTParser {
    fn process_type(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        parent_scope: Option<&str>,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let type_kind = match node.kind() {
            "class_declaration" => "class",
            "interface_declaration" => "interface",
            "trait_declaration" => "trait",
            "enum_declaration" => "enum",
            _ => "type",
        };

        let type_name = Self::extract_identifier(node, content)
            .unwrap_or_else(|| "Anonymous".to_string());

        let full_scope = match parent_scope {
            Some(parent) => format!("{} > {} {}", parent, type_kind, type_name),
            None => format!("{} {}", type_kind, type_name),
        };

        let mut start_b = node.start_byte();
        if let Some((c_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_b = c_start;
        }
        let end_b = node.end_byte();
        let full_code = &content[start_b..end_b];

        let body_node = node.child_by_field_name("body");

        // If the type fits within max_chunk_chars, emit it in one chunk
        if full_code.len() <= max_chunk_chars || body_node.is_none() {
            let start_line = content[..start_b].lines().count().max(1);
            let end_line = content[..end_b].lines().count().max(start_line);

            self.emit_or_split_code_chunk(
                file_path,
                file_name,
                &full_scope,
                &full_scope,
                full_code,
                start_line,
                end_line,
                chunks,
                max_chunk_chars,
            );
            return;
        }

        // Decompose large type into Overview + individual methods
        let body = body_node.unwrap();
        let body_start = body.start_byte();

        // 1. Overview chunk
        let header_slice = content[start_b..body_start].trim_end();
        let overview_text = format!("{} {{\n    // ... methods chunked separately ...\n}}", header_slice);
        let start_line = content[..start_b].lines().count().max(1);
        let end_line = content[..body_start].lines().count().max(start_line);

        let overview_header = format!("{} > Overview", full_scope);
        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &overview_header,
            &overview_header,
            &overview_text,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );

        // 2. Members inside declaration_list
        let mut last_inner_byte = body_start;
        for i in 0..body.child_count() {
            let member = body.child(i).unwrap();
            let kind = member.kind();

            if kind == "method_declaration" {
                let mut m_start = member.start_byte();
                if let Some((c_start, _)) = self.find_preceding_doc_comment(member, content) {
                    m_start = c_start;
                }

                if m_start > last_inner_byte {
                    let gap_text = content[last_inner_byte..m_start].trim();
                    if !gap_text.is_empty() {
                        let gap_start_line = content[..last_inner_byte].lines().count().max(1);
                        let gap_end_line = content[..m_start].lines().count().max(gap_start_line);
                        let gap_scope = format!("{} > Properties & Constants", full_scope);
                        self.emit_or_split_code_chunk(
                            file_path,
                            file_name,
                            &gap_scope,
                            &gap_scope,
                            gap_text,
                            gap_start_line,
                            gap_end_line,
                            chunks,
                            max_chunk_chars,
                        );
                    }
                }

                self.process_method(
                    file_path,
                    file_name,
                    content,
                    member,
                    Some(&full_scope),
                    chunks,
                    max_chunk_chars,
                );
                last_inner_byte = member.end_byte();
            }
        }

        // Remaining properties / constants at end of class body
        let body_end = body.end_byte();
        if last_inner_byte < body_end {
            let tail = content[last_inner_byte..body_end].trim();
            if !tail.is_empty() && tail != "}" {
                let tail_start_line = content[..last_inner_byte].lines().count().max(1);
                let tail_end_line = content[..body_end].lines().count().max(tail_start_line);
                let tail_scope = format!("{} > Declarations", full_scope);
                self.emit_or_split_code_chunk(
                    file_path,
                    file_name,
                    &tail_scope,
                    &tail_scope,
                    tail,
                    tail_start_line,
                    tail_end_line,
                    chunks,
                    max_chunk_chars,
                );
            }
        }
    }

    fn process_method(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        parent_scope: Option<&str>,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let method_name = Self::extract_identifier(node, content)
            .unwrap_or_else(|| "anonymous_method".to_string());

        let label = format!("function {}", method_name);
        let (header_path, breadcrumb_label) = match parent_scope {
            Some(scope) => (
                format!("{} > {}", scope, label),
                format!("{} > {}", scope, label),
            ),
            None => (label.clone(), label),
        };

        let mut start_b = node.start_byte();
        if let Some((c_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_b = c_start;
        }
        let end_b = node.end_byte();

        let code = &content[start_b..end_b];
        let start_line = content[..start_b].lines().count().max(1);
        let end_line = content[..end_b].lines().count().max(start_line);

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &header_path,
            &breadcrumb_label,
            code,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }

    fn process_function(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        parent_scope: Option<&str>,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let fn_name = Self::extract_identifier(node, content)
            .unwrap_or_else(|| "anonymous_function".to_string());

        let label = format!("function {}", fn_name);
        let (header_path, breadcrumb_label) = match parent_scope {
            Some(scope) => (
                format!("{} > {}", scope, label),
                format!("{} > {}", scope, label),
            ),
            None => (label.clone(), label),
        };

        let mut start_b = node.start_byte();
        if let Some((c_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_b = c_start;
        }
        let end_b = node.end_byte();

        let code = &content[start_b..end_b];
        let start_line = content[..start_b].lines().count().max(1);
        let end_line = content[..end_b].lines().count().max(start_line);

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &header_path,
            &breadcrumb_label,
            code,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_class_and_methods() {
        let code = r#"<?php

namespace App\Services;

use App\Models\User;

/**
 * Handles user management operations.
 */
class UserService {
    private UserRepository $repo;

    public function findUser(int $id): ?User {
        return $this->repo->find($id);
    }
}

function globalHelper(): string {
    return "ok";
}
"#;
        let parser = PhpASTParser::new();
        let chunks = parser.parse_chunks("UserService.php", code, 1800).unwrap();

        assert!(!chunks.is_empty());
        let class_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("class UserService"));
        assert!(class_chunk.is_some());
        assert_eq!(class_chunk.unwrap().content_type, "php");

        let fn_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("function globalHelper"));
        assert!(fn_chunk.is_some());
    }
}
