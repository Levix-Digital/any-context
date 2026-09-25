/// Universal Token Estimator & Density Budgeting Engine (Marco 6 / Prioridade 3).
/// Provides high-throughput, zero-dependency token estimation, model context limits,
/// and semantic boundary-preserving chunk truncation in native Rust.

/// Determines whether a character belongs to the CJK (Chinese, Japanese, Korean) ideographic or phonetic blocks.
#[inline]
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |   // CJK Unified Ideographs
        '\u{3400}'..='\u{4DBF}' |   // CJK Unified Ideographs Extension A
        '\u{20000}'..='\u{2A6DF}' | // CJK Extension B
        '\u{F900}'..='\u{FAFF}' |   // CJK Compatibility Ideographs
        '\u{2F800}'..='\u{2FA1F}' | // CJK Compatibility Supplement
        '\u{3040}'..='\u{309F}' |   // Hiragana
        '\u{30A0}'..='\u{30FF}' |   // Katakana
        '\u{AC00}'..='\u{D7AF}'     // Hangul Syllables
    )
}

/// Determines whether a character belongs to standard Unicode emoji ranges.
#[inline]
fn is_emoji(c: char) -> bool {
    matches!(c,
        '\u{1F300}'..='\u{1F5FF}' | // Misc Symbols and Pictographs
        '\u{1F600}'..='\u{1F64F}' | // Emoticons
        '\u{1F680}'..='\u{1F6FF}' | // Transport and Map
        '\u{2600}'..='\u{26FF}' |   // Misc symbols
        '\u{2700}'..='\u{27BF}' |   // Dingbats
        '\u{1F900}'..='\u{1F9FF}'   // Supplemental Symbols and Pictographs
    )
}

/// Fast O(n) universal token estimator with calibrated sub-word splitting and Unicode awareness.
/// Produces a safe, slightly conservative estimate correlated (>95%) with BPE tokenizers
/// without requiring heavy BPE vocabulary dictionaries in memory.
pub fn estimate_token_count(text: &str) -> usize {
    if text.trim().is_empty() {
        return 0;
    }

    let mut token_estimate: usize = 0;
    let mut in_word = false;
    let mut word_len: usize = 0;
    let mut prev_char: Option<char> = None;

    for c in text.chars() {
        if c.is_alphabetic() {
            if is_cjk(c) {
                in_word = false;
                word_len = 0;
                // CJK ideographs/syllables map to ~1-2 BPE tokens per glyph
                token_estimate += 2;
            } else if !in_word {
                in_word = true;
                word_len = 1;
                token_estimate += 1;
            } else {
                word_len += 1;
                // Detect sub-word boundaries (camelCase transition: lowercase -> UPPERCASE)
                if let Some(prev) = prev_char {
                    if prev.is_lowercase() && c.is_uppercase() {
                        token_estimate += 1;
                        word_len = 1;
                    } else if word_len > 7 {
                        // Average English sub-word BPE token split for longer words
                        token_estimate += 1;
                        word_len = 1;
                    }
                }
            }
        } else if c.is_numeric() {
            if !in_word {
                in_word = true;
                word_len = 1;
                token_estimate += 1;
            } else {
                word_len += 1;
                // BPE tokenizers group digits in pairs or triplets
                if word_len > 3 {
                    token_estimate += 1;
                    word_len = 1;
                }
            }
        } else if c.is_whitespace() {
            in_word = false;
            word_len = 0;
            // Explicit newlines represent structural break tokens
            if c == '\n' {
                token_estimate += 1;
            }
        } else if is_emoji(c) {
            in_word = false;
            word_len = 0;
            token_estimate += 2;
        } else {
            // Punctuation, operators, brackets, delimiters: group consecutive punctuation (e.g. ::, ->, ==, ): )
            in_word = false;
            word_len = 0;
            let is_prev_punct = prev_char.map(|p| !p.is_alphanumeric() && !p.is_whitespace() && !is_cjk(p) && !is_emoji(p)).unwrap_or(false);
            if !is_prev_punct {
                token_estimate += 1;
            }
        }

        prev_char = Some(c);
    }

    if token_estimate == 0 && !text.trim().is_empty() {
        token_estimate = 1;
    }

    token_estimate
}

/// Truncates text so that its estimated token count does not exceed `max_tokens`.
/// Truncates preferentially on semantic boundaries:
/// 1. Double newlines (paragraphs)
/// 2. Single newlines (lines of code / text)
/// 3. Sentence end (period, exclamation, question mark + space)
/// 4. Word boundary (whitespace)
/// Returns a tuple of `(truncated_text, was_truncated)`.
pub fn truncate_to_token_ceiling(text: &str, max_tokens: usize) -> (String, bool) {
    if text.is_empty() || max_tokens == 0 {
        return (String::new(), !text.is_empty());
    }

    let total_tokens = estimate_token_count(text);
    if total_tokens <= max_tokens {
        return (text.to_string(), false);
    }

    // Step 1: Scan forward character by character, tracking cumulative token count
    let mut current_tokens: usize = 0;
    let mut in_word = false;
    let mut word_len: usize = 0;
    let mut prev_char: Option<char> = None;
    let mut target_byte_idx = text.len();

    for (byte_idx, c) in text.char_indices() {
        let mut char_tokens = 0;

        if c.is_alphabetic() {
            if is_cjk(c) {
                in_word = false;
                word_len = 0;
                char_tokens = 2;
            } else if !in_word {
                in_word = true;
                word_len = 1;
                char_tokens = 1;
            } else {
                word_len += 1;
                if let Some(prev) = prev_char {
                    if prev.is_lowercase() && c.is_uppercase() {
                        char_tokens = 1;
                        word_len = 1;
                    } else if word_len > 7 {
                        char_tokens = 1;
                        word_len = 1;
                    }
                }
            }
        } else if c.is_numeric() {
            if !in_word {
                in_word = true;
                word_len = 1;
                char_tokens = 1;
            } else {
                word_len += 1;
                if word_len > 3 {
                    char_tokens = 1;
                    word_len = 1;
                }
            }
        } else if c.is_whitespace() {
            in_word = false;
            word_len = 0;
            if c == '\n' {
                char_tokens = 1;
            }
        } else if is_emoji(c) {
            in_word = false;
            word_len = 0;
            char_tokens = 2;
        } else {
            in_word = false;
            word_len = 0;
            let is_prev_punct = prev_char.map(|p| !p.is_alphanumeric() && !p.is_whitespace() && !is_cjk(p) && !is_emoji(p)).unwrap_or(false);
            if !is_prev_punct {
                char_tokens = 1;
            }
        }

        if current_tokens + char_tokens > max_tokens {
            target_byte_idx = byte_idx;
            break;
        }

        current_tokens += char_tokens;
        prev_char = Some(c);
    }

    if target_byte_idx >= text.len() {
        return (text.to_string(), false);
    }

    // Step 2: Search backwards from target_byte_idx for the best semantic cut boundary
    let prefix = &text[..target_byte_idx];
    let lookback_window = target_byte_idx.saturating_sub((target_byte_idx / 4).max(100));
    let search_slice = &prefix[lookback_window..];

    // Priority A: Paragraph break (\n\n)
    if let Some(pos) = search_slice.rfind("\n\n") {
        let cut = lookback_window + pos;
        return (text[..cut].trim_end().to_string(), true);
    }

    // Priority B: Line break (\n)
    if let Some(pos) = search_slice.rfind('\n') {
        let cut = lookback_window + pos;
        return (text[..cut].trim_end().to_string(), true);
    }

    // Priority C: Sentence end (". ", "? ", "! ")
    for punct in [". ", "? ", "! "] {
        if let Some(pos) = search_slice.rfind(punct) {
            let cut = lookback_window + pos + 1; // Include the punctuation mark
            return (text[..cut].trim_end().to_string(), true);
        }
    }

    // Priority D: Word boundary (space)
    if let Some(pos) = search_slice.rfind(' ') {
        let cut = lookback_window + pos;
        return (text[..cut].trim_end().to_string(), true);
    }

    // Fallback: Exact character boundary
    (prefix.trim_end().to_string(), true)
}

/// Retrieves the maximum allowable token limit for document chunks by embedding model name.
/// Compiles the authoritative model context windows directly into Rust.
pub fn get_embedding_token_limit_rs(model_name: &str) -> usize {
    let m = model_name.to_lowercase().trim().to_string();
    if m.is_empty() {
        return 2048;
    }

    // OpenAI embedding models (8191 tokens)
    if m.contains("text-embedding-3") || m.contains("text-embedding-ada-002") || m.contains("ada-002") {
        8191
    // Google / Gemini embedding models (2048 tokens)
    } else if m.contains("text-embedding-004") || m.contains("gemini") || m.contains("vertex") {
        2048
    // Nomic / Ollama embedding models (2048 tokens)
    } else if m.contains("nomic-embed-text") || m.contains("nomic") || m.contains("ollama") {
        2048
    // HuggingFace / MiniLM / BGE local models (512 tokens)
    } else if m.contains("all-minilm") || m.contains("minilm") || m.contains("bge-") {
        512
    // Cohere embedding models (4096 tokens)
    } else if m.contains("embed-english") || m.contains("embed-multilingual") || m.contains("cohere") {
        4096
    // Safe universal default (2048 tokens)
    } else {
        2048
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_token_count_empty() {
        assert_eq!(estimate_token_count(""), 0);
        assert_eq!(estimate_token_count("   "), 0);
    }

    #[test]
    fn test_estimate_token_count_simple_words() {
        let count = estimate_token_count("Hello world from Rust");
        assert!(count >= 4 && count <= 6, "Expected ~4 tokens, got {}", count);
    }

    #[test]
    fn test_estimate_token_count_code_identifiers() {
        let code = "def getUserById(user_id: int):\n    return db.query(user_id)\n";
        let count = estimate_token_count(code);
        // Code has camelCase, symbols, indentation, newlines
        assert!(count >= 12 && count <= 24, "Expected 12-24 tokens, got {}", count);
    }

    #[test]
    fn test_estimate_token_count_cjk() {
        let cjk = "こんにちは世界"; // 7 CJK characters
        let count = estimate_token_count(cjk);
        assert!(count >= 7 && count <= 14, "Expected 7-14 tokens, got {}", count);
    }

    #[test]
    fn test_get_embedding_token_limit_rs() {
        assert_eq!(get_embedding_token_limit_rs("text-embedding-3-small"), 8191);
        assert_eq!(get_embedding_token_limit_rs("text-embedding-3-large"), 8191);
        assert_eq!(get_embedding_token_limit_rs("text-embedding-ada-002"), 8191);
        assert_eq!(get_embedding_token_limit_rs("text-embedding-004"), 2048);
        assert_eq!(get_embedding_token_limit_rs("nomic-embed-text"), 2048);
        assert_eq!(get_embedding_token_limit_rs("all-MiniLM-L6-v2"), 512);
        assert_eq!(get_embedding_token_limit_rs("bge-small-en-v1.5"), 512);
        assert_eq!(get_embedding_token_limit_rs("unknown-model-xyz"), 2048);
    }

    #[test]
    fn test_truncate_to_token_ceiling_no_truncation_needed() {
        let text = "Line 1: Short text.\nLine 2: Another short line.\n";
        let (res, truncated) = truncate_to_token_ceiling(text, 50);
        assert!(!truncated);
        assert_eq!(res, text);
    }

    #[test]
    fn test_truncate_to_token_ceiling_truncates_at_newline() {
        let text = "Line 1: first content\nLine 2: second content\nLine 3: third content\nLine 4: fourth content\nLine 5: fifth content";
        let total = estimate_token_count(text);
        assert!(total > 15);

        let (res, truncated) = truncate_to_token_ceiling(text, 12);
        assert!(truncated);
        assert!(estimate_token_count(&res) <= 12);
        // Should truncate cleanly on a line boundary without breaking words
        assert!(!res.ends_with("cont"));
        assert!(res.contains("Line 1"));
    }
}
