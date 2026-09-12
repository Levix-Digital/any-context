use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct TypeScriptASTParser;

impl TypeScriptASTParser {
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
            "tsx" | "jsx" => (tree_sitter_typescript::LANGUAGE_TSX.into(), "typescript"),
            "js" | "mjs" | "cjs" => (tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), "javascript"),
            _ => (tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), "typescript"),
        }
    }

    fn extract_identifier(node: Node, content: &str) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(content[n.start_byte()..n.end_byte()].to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            match child.kind() {
                "identifier" | "property_identifier" | "type_identifier" => {
                    return Some(content[child.start_byte()..child.end_byte()].to_string());
                }
                "variable_declarator" => {
                    if let Some(n) = child.child_by_field_name("name") {
                        return Some(content[n.start_byte()..n.end_byte()].to_string());
                    }
                    for j in 0..child.child_count() {
                        let grand = child.child(j).unwrap();
                        if grand.kind() == "identifier" {
                            return Some(content[grand.start_byte()..grand.end_byte()].to_string());
                        }
                    }
                }
                _ => {}
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
                if c_text.starts_with("/**") || c_text.starts_with("//") {
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

    fn process_function(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        outer_start_byte: usize,
        outer_end_byte: usize,
        class_name: Option<&str>,
        content_type: &str,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let func_name = Self::extract_identifier(node, content).unwrap_or_else(|| "anonymous".to_string());
        let mut start_byte = outer_start_byte.min(node.start_byte());
        let end_byte = outer_end_byte.max(node.end_byte());

        if class_name.is_none() {
            if let Some((doc_start, _)) = self.find_preceding_doc_comment(node, content) {
                start_byte = doc_start;
            }
        }

        let fn_text = content[start_byte..end_byte].trim();
        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);

        let (header_path, breadcrumb_label) = match class_name {
            Some(cls) => (
                format!("class {} > method {}", cls, func_name),
                format!("class {} > method {}", cls, func_name),
            ),
            None => (format!("function {}", func_name), format!("function {}", func_name)),
        };

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &header_path,
            &breadcrumb_label,
            fn_text,
            start_line,
            end_line,
            content_type,
            chunks,
            max_chunk_chars,
        );
    }

    fn process_class(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        outer_start_byte: usize,
        outer_end_byte: usize,
        content_type: &str,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let class_name = Self::extract_identifier(node, content).unwrap_or_else(|| "AnonymousClass".to_string());
        let mut start_byte = outer_start_byte.min(node.start_byte());
        let end_byte = outer_end_byte.max(node.end_byte());

        if let Some((doc_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_byte = doc_start;
        }

        let class_text = content[start_byte..end_byte].trim();
        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);

        let header_path = format!("class {}", class_name);
        let breadcrumb_label = format!("class {}", class_name);

        if class_text.len() <= max_chunk_chars {
            self.emit_single_chunk(
                file_path,
                file_name,
                &header_path,
                &breadcrumb_label,
                class_text,
                start_line,
                end_line,
                content_type,
                chunks,
            );
            return;
        }

        // Oversized class: create Overview + split methods
        let body_node = (0..node.child_count())
            .map(|i| node.child(i).unwrap())
            .find(|c| c.kind() == "class_body");

        if let Some(body) = body_node {
            let header_end_byte = body.start_byte().min(end_byte);
            let header_text = content[start_byte..header_end_byte].trim();
            let ov_header = format!("class {} (Overview)", class_name);
            let ov_crumb = format!("class {} [Overview]", class_name);
            let ov_end_line = start_line + header_text.lines().count().saturating_sub(1);

            self.emit_single_chunk(
                file_path,
                file_name,
                &ov_header,
                &ov_crumb,
                &format!("{}\n  // ... (methods split into dedicated chunks)", header_text),
                start_line,
                ov_end_line,
                content_type,
                chunks,
            );

            for i in 0..body.child_count() {
                let member = body.child(i).unwrap();
                match member.kind() {
                    "method_definition" => {
                        self.process_function(
                            file_path,
                            file_name,
                            content,
                            member,
                            member.start_byte(),
                            member.end_byte(),
                            Some(&class_name),
                            content_type,
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
                class_text,
                start_line,
                end_line,
                content_type,
                chunks,
                max_chunk_chars,
            );
        }
    }

    fn process_type_or_interface(
        &self,
        file_path: &str,
        file_name: &str,
        content: &str,
        node: Node,
        outer_start_byte: usize,
        outer_end_byte: usize,
        kind_label: &str,
        content_type: &str,
        chunks: &mut Vec<ChunkPayload>,
        max_chunk_chars: usize,
    ) {
        let name = Self::extract_identifier(node, content).unwrap_or_else(|| "Anonymous".to_string());
        let mut start_byte = outer_start_byte.min(node.start_byte());
        let end_byte = outer_end_byte.max(node.end_byte());

        if let Some((doc_start, _)) = self.find_preceding_doc_comment(node, content) {
            start_byte = doc_start;
        }

        let item_text = content[start_byte..end_byte].trim();
        let start_line = content[..start_byte].lines().count().max(1);
        let end_line = content[..end_byte].lines().count().max(start_line);

        let header_path = format!("{} {}", kind_label, name);
        let breadcrumb_label = format!("{} {}", kind_label, name);

        self.emit_or_split_code_chunk(
            file_path,
            file_name,
            &header_path,
            &breadcrumb_label,
            item_text,
            start_line,
            end_line,
            content_type,
            chunks,
            max_chunk_chars,
        );
    }
}

impl Default for TypeScriptASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for TypeScriptASTParser {
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
        let (language, content_type) = Self::select_language(file_path);
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load TypeScript/JavaScript grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse TypeScript/JavaScript code".to_string())?;

        let root_node = tree.root_node();
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(file_path);

        let mut chunks = Vec::new();
        let mut last_covered_byte = 0;

        let child_count = root_node.child_count();
        for i in 0..child_count {
            let outer_node = root_node.child(i).unwrap();
            let mut node = outer_node;
            let mut kind = node.kind();
            let chunk_start_byte = outer_node.start_byte();
            let chunk_end_byte = outer_node.end_byte();

            // Unwrap export_statement if wrapping a declaration
            if kind == "export_statement" {
                let mut found_inner = false;
                for j in 0..outer_node.child_count() {
                    let inner = outer_node.child(j).unwrap();
                    let ikind = inner.kind();
                    if matches!(
                        ikind,
                        "function_declaration"
                            | "generator_function_declaration"
                            | "class_declaration"
                            | "interface_declaration"
                            | "type_alias_declaration"
                            | "enum_declaration"
                            | "lexical_declaration"
                            | "variable_declaration"
                    ) {
                        node = inner;
                        kind = ikind;
                        found_inner = true;
                        break;
                    }
                }
                if !found_inner {
                    continue;
                }
            }

            match kind {
                "function_declaration" | "generator_function_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        chunk_start_byte,
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_function(
                        file_path,
                        file_name,
                        content,
                        node,
                        chunk_start_byte,
                        chunk_end_byte,
                        None,
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = chunk_end_byte;
                }
                "class_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        chunk_start_byte,
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_class(
                        file_path,
                        file_name,
                        content,
                        node,
                        chunk_start_byte,
                        chunk_end_byte,
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = chunk_end_byte;
                }
                "interface_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        chunk_start_byte,
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_type_or_interface(
                        file_path,
                        file_name,
                        content,
                        node,
                        chunk_start_byte,
                        chunk_end_byte,
                        "interface",
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = chunk_end_byte;
                }
                "type_alias_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        chunk_start_byte,
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_type_or_interface(
                        file_path,
                        file_name,
                        content,
                        node,
                        chunk_start_byte,
                        chunk_end_byte,
                        "type",
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = chunk_end_byte;
                }
                "enum_declaration" => {
                    self.capture_preceding_gap(
                        file_path,
                        file_name,
                        content,
                        last_covered_byte,
                        chunk_start_byte,
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    self.process_type_or_interface(
                        file_path,
                        file_name,
                        content,
                        node,
                        chunk_start_byte,
                        chunk_end_byte,
                        "enum",
                        content_type,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = chunk_end_byte;
                }
                "lexical_declaration" | "variable_declaration" => {
                    let has_func = (0..node.child_count())
                        .map(|c| node.child(c).unwrap())
                        .any(|decl| {
                            decl.kind() == "variable_declarator"
                                && (0..decl.child_count()).any(|val| {
                                    let v = decl.child(val).unwrap();
                                    v.kind() == "arrow_function" || v.kind() == "function_expression"
                                })
                        });

                    if has_func {
                        self.capture_preceding_gap(
                            file_path,
                            file_name,
                            content,
                            last_covered_byte,
                            chunk_start_byte,
                            content_type,
                            &mut chunks,
                            max_chunk_chars,
                        );
                        self.process_type_or_interface(
                            file_path,
                            file_name,
                            content,
                            node,
                            chunk_start_byte,
                            chunk_end_byte,
                            "const",
                            content_type,
                            &mut chunks,
                            max_chunk_chars,
                        );
                        last_covered_byte = chunk_end_byte;
                    }
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
    fn test_typescript_functions_and_classes() {
        let code = r#"
import { useState } from 'react';

/** Sum numbers */
export function add(a: number, b: number): number {
    return a + b;
}

export const multiply = (a: number, b: number): number => {
    return a * b;
};

export class UserStore {
    private users: string[] = [];

    public addUser(name: string): void {
        this.users.push(name);
    }
}
"#;

        let parser = TypeScriptASTParser::new();
        let chunks = parser.parse_chunks("src/math.ts", code, 1800).unwrap();
        assert!(chunks.len() >= 3);
        assert!(chunks.iter().any(|c| c.text.contains("function add")));
        assert!(chunks.iter().any(|c| c.text.contains("const multiply")));
        assert!(chunks.iter().any(|c| c.text.contains("class UserStore")));
    }

    #[test]
    fn test_tsx_component() {
        let code = r#"
import React from 'react';

export const WelcomeBanner = ({ user }: { user: string }) => {
    return (
        <div className="banner">
            <h1>Welcome, {user}!</h1>
        </div>
    );
};
"#;

        let parser = TypeScriptASTParser::new();
        let chunks = parser.parse_chunks("src/WelcomeBanner.tsx", code, 1800).unwrap();
        assert!(!chunks.is_empty());
        assert!(chunks[0].text.contains("WelcomeBanner"));
    }
}
