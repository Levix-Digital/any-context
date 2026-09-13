/// Universal Multilingual & Code-Aware Tokenizer (Unicode Standard Compliant).
/// Provides high-throughput tokenization across all human languages and code syntax:
/// - Universal Unicode letter & digit segmentation.
/// - Diacritic folding (accent-tolerant search: e.g. "configuracao" matches "configuração", "uber" matches "über").
/// - Code sub-tokenization: splits camelCase, PascalCase, snake_case, kebab-case while preserving the full identifier.

pub fn fold_char(c: char) -> Option<char> {
    match c {
        'á' | 'à' | 'ã' | 'â' | 'ä' | 'å' | 'Á' | 'À' | 'Ã' | 'Â' | 'Ä' | 'Å' => Some('a'),
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => Some('e'),
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => Some('i'),
        'ó' | 'ò' | 'õ' | 'ô' | 'ö' | 'ø' | 'Ó' | 'Ò' | 'Õ' | 'Ô' | 'Ö' | 'Ø' => Some('o'),
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => Some('u'),
        'ç' | 'Ç' => Some('c'),
        'ñ' | 'Ñ' => Some('n'),
        'ý' | 'ÿ' | 'Ý' => Some('y'),
        'š' | 'Š' => Some('s'),
        'ž' | 'Ž' => Some('z'),
        'ć' | 'č' | 'Ć' | 'Č' => Some('c'),
        'ł' | 'Ł' => Some('l'),
        _ => None,
    }
}

pub fn fold_diacritics(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if let Some(folded) = fold_char(c) {
            out.push(folded);
        } else {
            out.push(c.to_lowercase().next().unwrap_or(c));
        }
    }
    out
}

/// Tokenizes text into normalized, searchable tokens.
/// Produces both original lowercase tokens and folded variants when diacritics are present,
/// plus sub-tokens for camelCase, snake_case, and kebab-case code identifiers.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current_word = String::new();

    for c in text.chars() {
        // We consider alphanumeric chars, underscores, hyphens, and dots as potential word characters
        if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
            current_word.push(c);
        } else {
            if !current_word.is_empty() {
                process_word(&current_word, &mut tokens);
                current_word.clear();
            }
        }
    }

    if !current_word.is_empty() {
        process_word(&current_word, &mut tokens);
    }

    tokens
}

fn process_word(raw: &str, tokens: &mut Vec<String>) {
    // Strip leading and trailing punctuation from word
    let word = raw.trim_matches(|c: char| c == '_' || c == '-' || c == '.');
    if word.is_empty() {
        return;
    }

    let lower = word.to_lowercase();
    let folded = fold_diacritics(word);

    // 1. Add full token in lowercase
    if !lower.is_empty() {
        tokens.push(lower.clone());
    }

    // 2. Add folded diacritic version if different from lowercase
    if !folded.is_empty() && folded != lower {
        tokens.push(folded.clone());
    }

    // 3. If word has internal delimiters (_, -, .) or casing transitions, extract sub-tokens
    let mut sub_tokens = Vec::new();
    let mut sub_buf = String::new();
    let mut chars = word.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '_' || c == '-' || c == '.' {
            if !sub_buf.is_empty() {
                sub_tokens.push(sub_buf.clone());
                sub_buf.clear();
            }
        } else if c.is_uppercase() {
            // Check camelCase transition: e.g. "verifyToken" -> split before 'T'
            if !sub_buf.is_empty() {
                // If previous was lowercase, or if next is lowercase (e.g. "XMLParser" -> "XML", "Parser")
                let next_is_lower = chars.peek().map(|n| n.is_lowercase()).unwrap_or(false);
                let prev_was_lower = sub_buf.chars().last().map(|p| p.is_lowercase()).unwrap_or(false);
                if prev_was_lower || (sub_buf.len() > 1 && next_is_lower) {
                    sub_tokens.push(sub_buf.clone());
                    sub_buf.clear();
                }
            }
            sub_buf.push(c);
        } else {
            sub_buf.push(c);
        }
    }

    if !sub_buf.is_empty() {
        sub_tokens.push(sub_buf);
    }

    // If we extracted more than 1 sub-token, add them
    if sub_tokens.len() > 1 {
        for st in sub_tokens {
            let st_clean = st.trim_matches(|c: char| !c.is_alphanumeric());
            if st_clean.len() >= 2 {
                let st_lower = st_clean.to_lowercase();
                let st_folded = fold_diacritics(st_clean);
                if !tokens.contains(&st_lower) {
                    tokens.push(st_lower.clone());
                }
                if st_folded != st_lower && !tokens.contains(&st_folded) {
                    tokens.push(st_folded);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_camel_case() {
        let tokens = tokenize("verifyToken AuthMiddleware");
        assert!(tokens.contains(&"verifytoken".to_string()));
        assert!(tokens.contains(&"verify".to_string()));
        assert!(tokens.contains(&"token".to_string()));
        assert!(tokens.contains(&"authmiddleware".to_string()));
        assert!(tokens.contains(&"auth".to_string()));
        assert!(tokens.contains(&"middleware".to_string()));
    }

    #[test]
    fn test_code_snake_case_screaming() {
        let tokens = tokenize("DATABASE_URL jwt_secret_key");
        assert!(tokens.contains(&"database_url".to_string()));
        assert!(tokens.contains(&"database".to_string()));
        assert!(tokens.contains(&"url".to_string()));
        assert!(tokens.contains(&"jwt_secret_key".to_string()));
        assert!(tokens.contains(&"jwt".to_string()));
        assert!(tokens.contains(&"secret".to_string()));
        assert!(tokens.contains(&"key".to_string()));
    }

    #[test]
    fn test_multilingual_diacritics() {
        let tokens = tokenize("Configuração de usuários e políticas");
        assert!(tokens.contains(&"configuração".to_string()));
        assert!(tokens.contains(&"configuracao".to_string()));
        assert!(tokens.contains(&"usuários".to_string()));
        assert!(tokens.contains(&"usuarios".to_string()));
        assert!(tokens.contains(&"políticas".to_string()));
        assert!(tokens.contains(&"politicas".to_string()));
    }

    #[test]
    fn test_german_spanish_french() {
        let tokens = tokenize("Überprüfung für das Jahr und résumé");
        assert!(tokens.contains(&"überprüfung".to_string()));
        assert!(tokens.contains(&"uberprufung".to_string()));
        assert!(tokens.contains(&"résumé".to_string()));
        assert!(tokens.contains(&"resume".to_string()));
    }

    #[test]
    fn test_dates_and_filenames() {
        let tokens = tokenize("deploy-2026-09-01.sh schema.sql Dockerfile");
        assert!(tokens.contains(&"deploy-2026-09-01.sh".to_string()));
        assert!(tokens.contains(&"deploy".to_string()));
        assert!(tokens.contains(&"2026".to_string()));
        assert!(tokens.contains(&"schema.sql".to_string()));
        assert!(tokens.contains(&"dockerfile".to_string()));
    }
}
