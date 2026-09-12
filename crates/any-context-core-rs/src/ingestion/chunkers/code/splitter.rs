pub struct SplitCodePart {
    pub text: String,
    pub start_line_offset: usize,
    pub end_line_offset: usize,
}

/// Splits any oversized code string into smaller chunks that strictly do not exceed max_chunk_chars.
/// Uses a 3-tier hierarchical strategy:
///   Tier 1: Paragraphs (double newline \n\n)
///   Tier 2: Lines (single newline \n)
///   Tier 3: Character window with safe delimiter boundary for minified or ultra-long lines
pub fn split_oversized_code(code: &str, max_chunk_chars: usize) -> Vec<SplitCodePart> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if trimmed.len() <= max_chunk_chars {
        let total_lines = trimmed.lines().count().max(1);
        return vec![SplitCodePart {
            text: trimmed.to_string(),
            start_line_offset: 0,
            end_line_offset: total_lines.saturating_sub(1),
        }];
    }

    // Deconstruct code into atomic units that are each guaranteed <= max_chunk_chars
    struct AtomicUnit {
        text: String,
        line_count: usize,
    }

    let mut units: Vec<AtomicUnit> = Vec::new();
    let paragraphs: Vec<&str> = trimmed.split("\n\n").collect();

    for (p_idx, p) in paragraphs.iter().enumerate() {
        let p_trimmed = p.trim();
        if p_trimmed.is_empty() {
            continue;
        }

        if p_trimmed.len() <= max_chunk_chars {
            // Add paragraph directly
            let line_count = p_trimmed.lines().count().max(1) + if p_idx > 0 { 1 } else { 0 };
            units.push(AtomicUnit {
                text: p_trimmed.to_string(),
                line_count,
            });
        } else {
            // Tier 2: Paragraph is too large. Break down into lines.
            let lines: Vec<&str> = p_trimmed.lines().collect();
            for l in lines {
                let l_trimmed = l.trim_end();
                if l_trimmed.is_empty() {
                    continue;
                }

                if l_trimmed.len() <= max_chunk_chars {
                    units.push(AtomicUnit {
                        text: l_trimmed.to_string(),
                        line_count: 1,
                    });
                } else {
                    // Tier 3: Single line is too large (e.g. minified JS, gigantic string literal)
                    let mut remaining = l_trimmed;
                    while !remaining.is_empty() {
                        if remaining.len() <= max_chunk_chars {
                            units.push(AtomicUnit {
                                text: remaining.to_string(),
                                line_count: 1,
                            });
                            break;
                        }

                        // Try to find a natural boundary near the cut-off
                        let search_window = &remaining[..max_chunk_chars];
                        let cut_point = search_window
                            .rfind(|c: char| c == ' ' || c == ';' || c == ',' || c == '{' || c == '}' || c == '(' || c == ')')
                            .map(|idx| idx + 1)
                            .unwrap_or(max_chunk_chars);

                        let cut = if cut_point == 0 { max_chunk_chars } else { cut_point };
                        let chunk_slice = remaining[..cut].trim();
                        if !chunk_slice.is_empty() {
                            units.push(AtomicUnit {
                                text: chunk_slice.to_string(),
                                line_count: 1,
                            });
                        }
                        remaining = remaining[cut..].trim_start();
                    }
                }
            }
        }
    }

    // Now assemble atomic units greedily into chunks <= max_chunk_chars
    let mut result: Vec<SplitCodePart> = Vec::new();
    let mut current_text = String::new();
    let mut current_start_line = 0;

    for u in units {
        let separator = if current_text.is_empty() {
            ""
        } else if u.line_count > 1 {
            "\n\n"
        } else {
            "\n"
        };

        let proposed_len = current_text.len() + separator.len() + u.text.len();
        if proposed_len > max_chunk_chars && !current_text.is_empty() {
            let part_lines = current_text.lines().count().max(1);
            let end_line = current_start_line + part_lines.saturating_sub(1);
            result.push(SplitCodePart {
                text: current_text.trim().to_string(),
                start_line_offset: current_start_line,
                end_line_offset: end_line,
            });

            current_start_line = end_line + 1;
            current_text.clear();
        }

        if !current_text.is_empty() {
            current_text.push_str(separator);
        }
        current_text.push_str(&u.text);
    }

    if !current_text.is_empty() {
        let part_lines = current_text.lines().count().max(1);
        let end_line = current_start_line + part_lines.saturating_sub(1);
        result.push(SplitCodePart {
            text: current_text.trim().to_string(),
            start_line_offset: current_start_line,
            end_line_offset: end_line,
        });
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_code_not_split() {
        let code = "fn small() {\n    println!(\"hello\");\n}";
        let parts = split_oversized_code(code, 1000);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].text, code);
    }

    #[test]
    fn test_split_by_single_newline_when_no_double_newline() {
        // Simulates continuous table-driven test cases without \n\n
        let mut continuous_code = String::new();
        for i in 0..100 {
            continuous_code.push_str(&format!("case_{}: assert_eq!(calc({}), {});\n", i, i, i * 2));
        }
        assert!(continuous_code.len() > 3000);

        let parts = split_oversized_code(&continuous_code, 500);
        assert!(parts.len() > 1);
        for part in &parts {
            assert!(part.text.len() <= 500, "Part length {} exceeds 500", part.text.len());
        }
    }

    #[test]
    fn test_split_ultra_long_single_line() {
        // Simulates minified JS or massive literal on single line
        let long_line = "var a=1;var b=2;var c=3;".repeat(100);
        assert!(long_line.len() > 2000);

        let parts = split_oversized_code(&long_line, 400);
        assert!(parts.len() > 1);
        for part in &parts {
            assert!(part.text.len() <= 400, "Part length {} exceeds 400", part.text.len());
        }
    }
}
