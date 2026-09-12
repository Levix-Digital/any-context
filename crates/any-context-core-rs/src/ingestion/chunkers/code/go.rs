use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct GoASTParser;

impl GoASTParser {
    pub fn new() -> Self {
        Self
    }

    fn extract_identifier(node: Node, content: &str) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(content[n.start_byte()..n.end_byte()].to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            if child.kind() == "identifier" || child.kind() == "type_identifier" || child.kind() == "field_identifier" {
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
            content_type: "go".to_string(),
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
}

impl Default for GoASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for GoASTParser {
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
        let language: Language = tree_sitter_go::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load Go grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse Go code".to_string())?;

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

                    let fn_text = content[start_byte..end_byte].trim();
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);
                    let label = format!("function {}", fn_name);

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
                "method_declaration" => {
                    let method_name = Self::extract_identifier(child, content)
                        .unwrap_or_else(|| "unnamed".to_string());
                    let receiver = child
                        .child_by_field_name("receiver")
                        .map(|r| content[r.start_byte()..r.end_byte()].trim())
                        .unwrap_or("");

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

                    let method_text = content[start_byte..end_byte].trim();
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);
                    let label = if !receiver.is_empty() {
                        format!("method {} {}", receiver, method_name)
                    } else {
                        format!("method {}", method_name)
                    };

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &label,
                        &label,
                        method_text,
                        start_line,
                        end_line,
                        &mut chunks,
                        max_chunk_chars,
                    );

                    last_covered_byte = end_byte;
                }
                "type_declaration" => {
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

                    // Determine type name and kind label
                    let mut type_name = "Anonymous".to_string();
                    let mut kind_label = "type";

                    for j in 0..child.child_count() {
                        let c = child.child(j).unwrap();
                        if c.kind() == "type_spec" {
                            if let Some(n) = c.child_by_field_name("name") {
                                type_name = content[n.start_byte()..n.end_byte()].to_string();
                            }
                            if let Some(t) = c.child_by_field_name("type") {
                                match t.kind() {
                                    "struct_type" => kind_label = "struct",
                                    "interface_type" => kind_label = "interface",
                                    _ => kind_label = "type",
                                }
                            }
                            break;
                        }
                    }

                    let type_text = content[start_byte..end_byte].trim();
                    let start_line = content[..start_byte].lines().count().max(1);
                    let end_line = content[..end_byte].lines().count().max(start_line);
                    let label = format!("{} {}", kind_label, type_name);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &label,
                        &label,
                        type_text,
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
    fn test_go_functions_methods_and_types() {
        let code = r#"
package service

import (
    "context"
    "fmt"
)

// PaymentService handles payment processing.
type PaymentService struct {
    gatewayURL string
}

// PaymentGateway interface defines payment contract.
type PaymentGateway interface {
    ProcessPayment(ctx context.Context, amount float64) (string, error)
}

// Process handles an order payment.
func (s *PaymentService) Process(ctx context.Context, amount float64) error {
    if amount <= 0 {
        return fmt.Errorf("invalid amount: %f", amount)
    }
    return nil
}

// NewPaymentService creates a new service instance.
func NewPaymentService(url string) *PaymentService {
    return &PaymentService{gatewayURL: url}
}
"#;

        let parser = GoASTParser::new();
        let chunks = parser.parse_chunks("src/service.go", code, 1800).unwrap();
        assert!(!chunks.is_empty());

        // Check module gap (package + imports)
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("module") && c.text.contains("package service")));

        // Check struct
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("struct PaymentService")));

        // Check interface
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("interface PaymentGateway")));

        // Check method with receiver
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("method (s *PaymentService) Process")));

        // Check standalone function
        assert!(chunks.iter().any(|c| c.header_path.as_deref() == Some("function NewPaymentService")));
    }
}
