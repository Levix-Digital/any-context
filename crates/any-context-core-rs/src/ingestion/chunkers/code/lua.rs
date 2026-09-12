use crate::ingestion::chunkers::code::traits::LanguageASTParser;
use crate::models::ChunkPayload;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::path::Path;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone)]
pub struct LuaASTParser;

impl LuaASTParser {
    pub fn new() -> Self {
        Self
    }

    fn extract_function_name(node: Node, content: &str) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(content[n.start_byte()..n.end_byte()].trim().to_string());
        }
        for i in 0..node.child_count() {
            let child = node.child(i).unwrap();
            let k = child.kind();
            if k == "identifier" || k == "dot_index_expression" || k == "method_index_expression" {
                return Some(content[child.start_byte()..child.end_byte()].trim().to_string());
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
                if c_text.starts_with("--") {
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
            content_type: "lua".to_string(),
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
            "Module Setup & Script",
            "setup",
            trimmed,
            start_line,
            end_line,
            chunks,
            max_chunk_chars,
        );
    }
}

impl Default for LuaASTParser {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageASTParser for LuaASTParser {
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
        let language: Language = tree_sitter_lua::LANGUAGE.into();
        parser
            .set_language(&language)
            .map_err(|e| format!("Failed to load Lua grammar: {}", e))?;

        let tree = parser
            .parse(content, None)
            .ok_or_else(|| "Failed to parse Lua code".to_string())?;

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

                    let is_local = content[child.start_byte()..child.end_byte()].trim_start().starts_with("local ");
                    let fn_name = Self::extract_function_name(child, content)
                        .unwrap_or_else(|| "anonymous_function".to_string());
                    let prefix = if is_local { "local function" } else { "function" };
                    let label = format!("{} {}", prefix, fn_name);

                    let end_b = child.end_byte();
                    let code = &content[start_b..end_b];
                    let start_line = content[..start_b].lines().count().max(1);
                    let end_line = content[..end_b].lines().count().max(start_line);

                    self.emit_or_split_code_chunk(
                        file_path,
                        file_name,
                        &label,
                        &label,
                        code,
                        start_line,
                        end_line,
                        &mut chunks,
                        max_chunk_chars,
                    );
                    last_covered_byte = child.end_byte();
                }
                "local_declaration" => {
                    // Check if this local declaration is a local function definition
                    let mut inner_fn = None;
                    for j in 0..child.child_count() {
                        let sub = child.child(j).unwrap();
                        if sub.kind() == "function_declaration" {
                            inner_fn = Some(sub);
                            break;
                        }
                    }

                    if let Some(f_node) = inner_fn {
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

                        let fn_name = Self::extract_function_name(f_node, content)
                            .unwrap_or_else(|| "anonymous_function".to_string());
                        let label = format!("local function {}", fn_name);

                        let end_b = child.end_byte();
                        let code = &content[start_b..end_b];
                        let start_line = content[..start_b].lines().count().max(1);
                        let end_line = content[..end_b].lines().count().max(start_line);

                        self.emit_or_split_code_chunk(
                            file_path,
                            file_name,
                            &label,
                            &label,
                            code,
                            start_line,
                            end_line,
                            &mut chunks,
                            max_chunk_chars,
                        );
                        last_covered_byte = child.end_byte();
                    }
                }
                "assignment_statement" => {
                    // Check if right-hand side contains a function_definition
                    let mut is_fn_assign = false;
                    let mut var_name = None;

                    for j in 0..child.child_count() {
                        let sub = child.child(j).unwrap();
                        if sub.kind() == "variable_list" {
                            var_name = Some(content[sub.start_byte()..sub.end_byte()].trim().to_string());
                        } else if sub.kind() == "expression_list" {
                            for k in 0..sub.child_count() {
                                if sub.child(k).unwrap().kind() == "function_definition" {
                                    is_fn_assign = true;
                                    break;
                                }
                            }
                        }
                    }

                    if is_fn_assign {
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

                        let name_str = var_name.unwrap_or_else(|| "anonymous".to_string());
                        let label = format!("function {}", name_str);

                        let end_b = child.end_byte();
                        let code = &content[start_b..end_b];
                        let start_line = content[..start_b].lines().count().max(1);
                        let end_line = content[..end_b].lines().count().max(start_line);

                        self.emit_or_split_code_chunk(
                            file_path,
                            file_name,
                            &label,
                            &label,
                            code,
                            start_line,
                            end_line,
                            &mut chunks,
                            max_chunk_chars,
                        );
                        last_covered_byte = child.end_byte();
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_functions_and_modules() {
        let code = r#"
local M = {}

-- Greet the user warmly
function M:greet(name)
    return "Hello, " .. name
end

local function internal_calc(x, y)
    return x * y + 42
end

return M
"#;
        let parser = LuaASTParser::new();
        let chunks = parser.parse_chunks("init.lua", code, 1800).unwrap();

        assert!(!chunks.is_empty());
        let greet_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("function M:greet"));
        assert!(greet_chunk.is_some());
        assert_eq!(greet_chunk.unwrap().content_type, "lua");

        let local_chunk = chunks.iter().find(|c| c.header_path.as_deref() == Some("local function internal_calc"));
        assert!(local_chunk.is_some());
    }
}
