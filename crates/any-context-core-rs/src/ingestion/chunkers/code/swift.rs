use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct SwiftASTParser;

impl SwiftASTParser {
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
            if k == "type_identifier" || k == "simple_identifier" || k == "identifier" {
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
                if c_text.starts_with("///") || c_text.starts_with("//") || c_text.starts_with("/**") || c_text.starts_with("/*") {
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
            content_type: "swift".to_string(),
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
            "Imports & Preamble",
            "imports",
            trimmed,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }
}

impl Default for SwiftASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for SwiftASTParser {
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
        let language: Language = tree_sitter_swift::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load Swift grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse Swift code".to_string())?;

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
                "class_declaration" | "struct_declaration" | "enum_declaration" | "protocol_declaration" | "extension_declaration" => {
                    let mut decl_label = match kind {
                        "struct_declaration" => "struct",
                        "enum_declaration" => "enum",
                        "protocol_declaration" => "protocol",
                        "extension_declaration" => "extension",
                        _ => "class",
                    };
                    if kind == "class_declaration" {
                        let inner_code = content[child.start_byte()..child.end_byte()].trim_start();
                        if inner_code.starts_with("struct ") || inner_code.starts_with("public struct ") || inner_code.starts_with("private struct ") || inner_code.starts_with("internal struct ") {
                            decl_label = "struct";
                        } else if inner_code.starts_with("actor ") || inner_code.starts_with("public actor ") || inner_code.starts_with("private actor ") || inner_code.starts_with("internal actor ") {
                            decl_label = "actor";
                        }
                    }

                    let type_name = Self::extract_identifier(child, content)
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

                    let decl_text = &content[start_byte..end_byte];
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);

                    if decl_text.len() > max_chunk_chars {
                        let body_node = child.child_by_field_name("body").or_else(|| {
                            for j in 0..child.child_count() {
                                let c = child.child(j).unwrap();
                                if c.kind() == "class_body" || c.kind() == "struct_body" || c.kind() == "enum_body" || c.kind() == "protocol_body" || c.kind() == "extension_body" {
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
                                &format!("{} {}", decl_label, type_name),
                                &format!("{} {} (Overview)", decl_label, type_name),
                                overview_text,
                                start_line,
                                overview_end_line,
                                &mut chunks,
                                max_chunk_chars,
                            );

                            for m_idx in 0..body.child_count() {
                                let member = body.child(m_idx).unwrap();
                                let m_kind = member.kind();

                                if m_kind == "function_declaration" || m_kind == "init_declaration" {
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
                                        .unwrap_or_else(|| if m_kind == "init_declaration" { "init".to_string() } else { "func".to_string() });

                                    let m_label = format!("func {}", m_name);

                                    self.emit_or_split_code_chunk(
                                        file_path,
                                        file_name,
                                        &format!("{} {} > {}", decl_label, type_name, m_label),
                                        &format!("{} {} > {}", decl_label, type_name, m_label),
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
                                &format!("{} {}", decl_label, type_name),
                                &format!("{} {}", decl_label, type_name),
                                decl_text,
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
                            &format!("{} {}", decl_label, type_name),
                            &format!("{} {}", decl_label, type_name),
                            decl_text,
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
                        &format!("func {}", fn_name),
                        &format!("func {}", fn_name),
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
    fn test_swift_struct_and_functions() {
        let code = r#"
import Foundation

/// Order representation in iOS client
struct OrderItem: Codable {
    let id: UUID
    let title: String
    let price: Double

    func calculateTax(rate: Double) -> Double {
        return price * rate
    }
}

func submitOrder(item: OrderItem) -> Bool {
    return true
}
"#;
        let parser = SwiftASTParser::new();
        let chunks = parser.parse_chunks("Order.swift", code, 1800).unwrap();

        assert!(!chunks.is_empty());
        let struct_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("struct OrderItem"));
        assert!(struct_chunk.is_some());
        assert_eq!(struct_chunk.unwrap().content_type, "swift");

        let fn_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("func submitOrder"));
        assert!(fn_chunk.is_some());
    }
}
