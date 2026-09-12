use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct RustASTParser;

impl RustASTParser {
    pub fn new() -> Self {
        Self
    }

    fn extract_identifier(node: Node, content: &str) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(content[n.start_byte()..n.end_byte()].to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind() == "identifier" || child.kind() == "type_identifier" {
                return Some(content[child.start_byte()..child.end_byte()].to_string());
            }
        }
        None
    }

    fn find_preceding_doc_comment<'a>(&self, node: Node, content: &'a str) -> Option<(usize, &'a str)> {
        let mut prev = node.prev_sibling();
        while let Some(p) = prev {
            let kind = p.kind();
            if kind == "comment" || kind == "line_comment" || kind == "block_comment" {
                let c_text = &content[p.start_byte()..p.end_byte()];
                if c_text.starts_with("///") || c_text.starts_with("//!") || c_text.starts_with("//") || c_text.starts_with("/*") {
                    return Some((p.start_byte(), c_text));
                }
            }
            if !p.is_extra() && kind != "comment" && kind != "attribute_item" && kind != "inner_attribute_item" {
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
            content_type: "rust".to_string(),
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

    fn process_impl_item(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let type_name = node
            .child_by_field_name("type")
            .map(|n| content[n.start_byte()..n.end_byte()].trim())
            .unwrap_or("Anonymous");

        let trait_name = node
            .child_by_field_name("trait")
            .map(|n| content[n.start_byte()..n.end_byte()].trim());

        let impl_label = if let Some(tr) = trait_name {
            format!("impl {} for {}", tr, type_name)
        } else {
            format!("impl {}", type_name)
        };

        let mut start_byte = node.start_byte();
        let end_byte = node.end_byte();

        if let Some((doc_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_byte = doc_start;
        }

        let item_text = content[start_byte..end_byte].trim();
        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);

        if item_text.len() <= max_chunk_chars {
            self.emit_single_chunk(
                file_path,
                file_name,
                &impl_label,
                &impl_label,
                item_text,
                start_line,
                end_line,
                chunks,
            );
            return;
        }

        // Oversized impl block: emit overview + split member functions
        let body_node = node.child_by_field_name("body");
        if let Some(body) = body_node {
            let header_end_byte = body.start_byte().min(end_byte);
            let header_text = content[start_byte..header_end_byte].trim();
            let ov_header = format!("{} (Overview)", impl_label);
            let ov_crumb = format!("{} [Overview]", impl_label);
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
                if member.kind() == "function_item" {
                    let fn_name = Self::extract_identifier(member, content)
                        .unwrap_or_else(|| "unnamed".to_string());
                    let mut fn_start = member.start_byte();
                    let fn_end = member.end_byte();

                    if let Some((doc_start, _)) = self.find_preceding_doc_comment(member, content) {
                        fn_start = doc_start;
                    }

                    let fn_text = content[fn_start..fn_end].trim();
                    let m_start_line = content[..fn_start].lines().count().max(1);
                    let m_end_line = content[..fn_end].lines().count().max(m_start_line);
                    let member_label = format!("{} > fn {}", impl_label, fn_name);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &member_label,
                        &member_label,
                        fn_text,
                        m_start_line,
                        m_end_line,
                        chunks,
                        max_chunk_chars,
                    );
                }
            }
        } else {
            self.emit_or_split_code_chunk(
                file_path,
                file_name,
                &impl_label,
                &impl_label,
                item_text,
                start_line,
                end_line,
                chunks,
                max_chunk_chars,
            );
        }
    }
}

impl Default for RustASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for RustASTParser {
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
        let language: Language = tree_sitter_rust::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load Rust grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse Rust code".to_string())?;

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
                "function_item" => {
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

                    let fn_text = content[start_byte..end_byte].trim();
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);
                    let label = format!("fn {}", fn_name);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &label,
                        &label,
                        fn_text,
                        start_line,
                        end_line,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    last_covered_byte = end_byte;
                }
                "struct_item" => {
                    let struct_name = Self::extract_identifier(child, content)
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

                    let item_text = content[start_byte..end_byte].trim();
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);
                    let label = format!("struct {}", struct_name);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &label,
                        &label,
                        item_text,
                        start_line,
                        end_line,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    last_covered_byte = end_byte;
                }
                "enum_item" => {
                    let enum_name = Self::extract_identifier(child, content)
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

                    let item_text = content[start_byte..end_byte].trim();
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);
                    let label = format!("enum {}", enum_name);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &label,
                        &label,
                        item_text,
                        start_line,
                        end_line,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    last_covered_byte = end_byte;
                }
                "trait_item" => {
                    let trait_name = Self::extract_identifier(child, content)
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

                    let item_text = content[start_byte..end_byte].trim();
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);
                    let label = format!("trait {}", trait_name);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &label,
                        &label,
                        item_text,
                        start_line,
                        end_line,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    last_covered_byte = end_byte;
                }
                "impl_item" => {
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

                    self.process_impl_item(
                        file_path,
                        file_name,
                        content,
                        child,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    last_covered_byte = end_byte;
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
    fn test_rust_struct_impl_trait_and_functions() {
        let code = r#"
//! Module level documentation.

use std::sync::Arc;

/// Represents an item in inventory.
#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub sku: String,
    pub quantity: u32,
}

/// Status of the order item.
pub enum ItemStatus {
    InStock,
    OutOfStock,
}

pub trait StockChecker {
    fn is_available(&self) -> bool;
}

impl InventoryItem {
    pub fn new(sku: String, quantity: u32) -> Self {
        Self { sku, quantity }
    }

    pub fn reserve(&mut self, qty: u32) -> bool {
        if self.quantity >= qty {
            self.quantity -= qty;
            true
        } else {
            false
        }
    }
}

impl StockChecker for InventoryItem {
    fn is_available(&self) -> bool {
        self.quantity > 0
    }
}

pub fn check_stock(item: &InventoryItem) -> bool {
    item.quantity > 0
}
"#;

        let parser = RustASTParser::new();
        let chunks = parser.parse_chunks("src/inventory.rs", code, 1800).unwrap();
        assert!(!chunks.is_empty());

        // Check module gap
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("module") && c.text.contains("use std::sync::Arc")));

        // Check struct
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("struct InventoryItem")));

        // Check enum
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("enum ItemStatus")));

        // Check trait
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("trait StockChecker")));

        // Check inherent impl
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("impl InventoryItem")));

        // Check trait impl
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("impl StockChecker for InventoryItem")));

        // Check standalone function
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("fn check_stock")));
    }
}
