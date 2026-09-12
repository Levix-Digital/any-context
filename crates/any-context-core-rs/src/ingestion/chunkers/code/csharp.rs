use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct CSharpASTParser;

impl CSharpASTParser {
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
            if kind == "comment" {
                let c_text = &content[p.start_byte()..p.end_byte()];
                if c_text.starts_with("///") || c_text.starts_with("/*") || c_text.starts_with("//") {
                    return Some((p.start_byte(), c_text));
                }
            }
            if !p.is_extra() && kind != "comment" && kind != "attribute_list" {
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
            content_type: "csharp".to_string(),
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

        // Oversized: create Overview + split members
        let body_node = (0..node.child_count())
            .map(|i| node.child(i).unwrap())
            .find(|c| c.kind() == "declaration_list");

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
                &format!("{}\n  // ... (members split into dedicated chunks)", header_text),
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
                    "property_declaration" => {
                        self.process_method(
                            file_path,
                            file_name,
                            content,
                            member,
                            &name,
                            "property",
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

    fn collect_declarations<'a>(
        &self,
        parent: Node<'a>,
        declarations: &mut Vec<Node<'a>>,
    ) {
        let count = parent.child_count();
        for i in 0..count {
            let child = parent.child(i).unwrap();
            let kind = child.kind();
            match kind {
                "class_declaration"
                | "interface_declaration"
                | "struct_declaration"
                | "record_declaration"
                | "enum_declaration" => {
                    declarations.push(child);
                }
                "namespace_declaration" | "file_scoped_namespace_declaration" => {
                    let inner_body = (0..child.child_count())
                        .map(|j| child.child(j).unwrap())
                        .find(|c| c.kind() == "declaration_list");

                    if let Some(body) = inner_body {
                        self.collect_declarations(body, declarations);
                    } else {
                        self.collect_declarations(child, declarations);
                    }
                }
                _ => {}
            }
        }
    }
}

impl Default for CSharpASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for CSharpASTParser {
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
        let language: Language = tree_sitter_c_sharp::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load C# grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse C# code".to_string())?;

        let root_node = tree.root_node();
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(file_path);

        let mut chunks = Vec::new();
        let mut last_covered_byte = 0;

        let mut declarations = Vec::new();
        self.collect_declarations(root_node, &mut declarations);

        for node in declarations {
            let kind = node.kind();
            let kind_label = match kind {
                "class_declaration" => "class",
                "interface_declaration" => "interface",
                "struct_declaration" => "struct",
                "record_declaration" => "record",
                "enum_declaration" => "enum",
                _ => "type",
            };

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
                kind_label,
                &mut chunks,
                max_chunk_chars,
            );
            last_covered_byte = node.end_byte();
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
    fn test_csharp_class_methods_and_properties() {
        let code = r#"
namespace ECommerce.Controllers;

using System;
using Microsoft.AspNetCore.Mvc;

[ApiController]
[Route("api/[controller]")]
public class OrdersController : ControllerBase
{
    private readonly IOrderService _service;

    public OrdersController(IOrderService service)
    {
        _service = service;
    }

    [HttpGet("{id}")]
    public IActionResult GetById(string id)
    {
        return Ok(_service.Get(id));
    }
}
"#;

        let parser = CSharpASTParser::new();
        let chunks = parser.parse_chunks("src/OrdersController.cs", code, 1800).unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("class OrdersController")));
    }
}
