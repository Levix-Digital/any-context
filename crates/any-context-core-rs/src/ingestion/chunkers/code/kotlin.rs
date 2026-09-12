use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct KotlinASTParser;

impl KotlinASTParser {
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
            if k == "identifier" || k == "simple_identifier" || k == "type_identifier" {
                return Some(content[child.start_byte()..child.end_byte()].to_string());
            }
        }
        None
    }

    fn find_preceding_doc_comment<'a>(&self, node: Node, content: &'a str) -> Option<(usize, &'a str)> {
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            let kind = p.kind();
            if kind == "comment" || kind == "multiline_comment" {
                let c_text = &content[p.start_byte()..p.end_byte()];
                if c_text.starts_with("//") || c_text.starts_with("/*") {
                    return Some((p.start_byte(), c_text));
                }
            }
            if !p.is_extra() && kind != "comment" && kind != "multiline_comment" {
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
            content_type: "kotlin".to_string(),
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
            "Package & Imports",
            "imports",
            trimmed,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }
}

impl Default for KotlinASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for KotlinASTParser {
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
        let language: Language = tree_sitter_kotlin_ng::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load Kotlin grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse Kotlin code".to_string())?;

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
                "class_declaration" | "object_declaration" => {
                    let is_object = kind == "object_declaration";
                    let type_label = if is_object { "object" } else { "class" };
                    let class_name = Self::extract_identifier(child, content)
                        .unwrap_or_else(|| "Anonymous".to_string());

                    let mut start_byte = child.start_byte();
                    let end_byte = child.end_byte();

                    if let Some((doc_start, _)) = self.find_preceding_doc_comment(child, content) {
                        start_byte = doc_start;
                    }

                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        start_byte,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    let class_text = &content[start_byte..end_byte];
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);

                    if class_text.len() > max_chunk_chars {
                        let body_node = child.child_by_field_name("body").or_else(|| {
                            for j in 0..child.child_count() {
                                let c = child.child(j).unwrap();
                                if c.kind() == "class_body" {
                                    return Some(c);
                                }
                            }
                            None
                        });

                        if let Some(body) = body_node {
                            let overview_end_byte = body.start_byte();
                            let overview_text = &content[start_byte..overview_end_byte].trim_end();
                            let overview_end_line = content[..overview_end_byte].lines().count().max(start_line);

                            self.emit_or_split_code_chunk(
                                file_path,
                                file_name,
                                &format!("{} {}", type_label, class_name),
                                &format!("{} {} (Overview)", type_label, class_name),
                                overview_text,
                                start_line,
                                overview_end_line,
                                &mut chunks,
                                max_chunk_chars,
                            );

                            for m_idx in 0..body.child_count() {
                                let member = body.child(m_idx).unwrap();
                                let m_kind = member.kind();

                                if m_kind == "function_declaration" || m_kind == "secondary_constructor" || m_kind == "class_declaration" {
                                    let mut m_start = member.start_byte();
                                    let m_end = member.end_byte();

                                    if let Some((m_doc, _)) = self.find_preceding_doc_comment(member, content) {
                                        if m_doc >= start_byte {
                                            m_start = m_doc;
                                        }
                                    }

                                    let member_code = &content[m_start..m_end];
                                    let m_start_line = content[..m_start].lines().count().max(1);
                                    let m_end_line = content[..m_end].lines().count().max(m_start_line);

                                    let m_name = Self::extract_identifier(member, content)
                                        .unwrap_or_else(|| "member".to_string());

                                    let m_label = if m_kind == "function_declaration" {
                                        format!("fun {}", m_name)
                                    } else if m_kind == "secondary_constructor" {
                                        "constructor".to_string()
                                    } else {
                                        format!("inner class {}", m_name)
                                    };

                                    self.emit_or_split_code_chunk(
                                        file_path,
                                        file_name,
                                        &format!("{} {} > {}", type_label, class_name, m_label),
                                        &format!("{} {} > {}", type_label, class_name, m_label),
                                        member_code,
                                        m_start_line,
                                        m_end_line,
                                        &mut chunks,
                                        max_chunk_chars,
                                    );
                                }
                            }
                        } else {
                            self.emit_or_split_code_chunk(
                                file_path,
                                file_name,
                                &format!("{} {}", type_label, class_name),
                                &format!("{} {}", type_label, class_name),
                                class_text,
                                start_line,
                                end_line,
                                &mut chunks,
                                max_chunk_chars,
                            );
                        }
                    } else {
                        self.emit_single_chunk(
                            file_path,
                            file_name,
                            &format!("{} {}", type_label, class_name),
                            &format!("{} {}", type_label, class_name),
                            class_text,
                            start_line,
                            end_line,
                            &mut chunks,
                        );
                    }

                    last_covered_byte = end_byte;
                }
                "function_declaration" => {
                    let fn_name = Self::extract_identifier(child, content)
                        .unwrap_or_else(|| "unnamed".to_string());

                    let mut start_byte = child.start_byte();
                    let end_byte = child.end_byte();

                    if let Some((doc_start, _)) = self.find_preceding_doc_comment(child, content) {
                        start_byte = doc_start;
                    }

                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        start_byte,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    let fn_text = &content[start_byte..end_byte];
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &format!("fun {}", fn_name),
                        &format!("fun {}", fn_name),
                        fn_text,
                        start_line,
                        end_line,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    last_covered_byte = end_byte;
                }
                _ => {}
            }
        }

        if last_covered_byte < content.len() {
            let trailing = &content[last_covered_byte..].trim();
            if !trailing.is_empty() {
                let start_line = content[..last_covered_byte].lines().count().max(1);
                let end_line = content.lines().count().max(start_line);

                self.emit_or_split_code_chunk(
                    file_path,
                    file_name,
                    "Module Tail",
                    "tail",
                    trailing,
                    start_line,
                    end_line,
                    &mut chunks,
                    max_chunk_chars,
                );
            }
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kotlin_class_and_methods() {
        let code = r#"
package com.example.service

import java.time.Instant

/**
 * Service for managing user profiles.
 */
class UserService(private val repo: UserRepository) {
    fun findById(id: Long): User? {
        return repo.findById(id)
    }

    fun deleteUser(id: Long) {
        repo.deleteById(id)
    }
}

fun helperFunction(): String {
    return "ok"
}
"#;
        let parser = KotlinASTParser::new();
        let chunks = parser.parse_chunks("UserService.kt", code, 1800).unwrap();

        assert!(!chunks.is_empty());
        let class_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("class UserService"));
        assert!(class_chunk.is_some());
        assert_eq!(class_chunk.unwrap().content_type, "kotlin");

        let helper_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("fun helperFunction"));
        assert!(helper_chunk.is_some());
    }
}
