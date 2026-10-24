//! Secret redaction before indexing (BUG-024, ADR-0149).
//!
//! Secret values (private keys, API keys, tokens, passwords) are detected by known patterns
//! and by entropy, as in the market's secret scanners, and replaced by [`SECRET_PLACEHOLDER`]
//! before any text is chunked, embedded or stored. Only the value is replaced: key names,
//! quotes and line breaks stay, so the rest of the file remains searchable and line numbers
//! do not move. There is no configuration and no allow list by design.

use regex::{Captures, Regex};
use std::sync::LazyLock;

/// Text that replaces every secret value.
pub const SECRET_PLACEHOLDER: &str = "[SECRET REDACTED]";

/// Result of [`redact_secrets`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redaction {
    /// The input with every secret value replaced by [`SECRET_PLACEHOLDER`].
    pub text: String,
    /// Number of values replaced.
    pub secrets_found: usize,
}

static PEM_BEGIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----").unwrap());
static PEM_END: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-----END [A-Z0-9 ]*PRIVATE KEY(?: BLOCK)?-----").unwrap());

/// Provider token formats with a fixed prefix and shape; the whole match is the secret.
static KNOWN_TOKENS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{36,}\b",
        r"\bgithub_pat_[A-Za-z0-9_]{22,}\b",
        r"\bglpat-[A-Za-z0-9_\-]{20,}",
        r"\bxox[abprs]-[A-Za-z0-9\-]{10,}",
        r"https://hooks\.slack\.com/services/[A-Za-z0-9/_\-]+",
        r"\b[sr]k_(?:live|test)_[A-Za-z0-9]{16,}\b",
        r"\bAIza[0-9A-Za-z_\-]{35}",
        r"\bsk-ant-[A-Za-z0-9_\-]{20,}",
        r"\bsk-(?:proj-|svcacct-|admin-)?[A-Za-z0-9_\-]{20,}",
        r"\bhf_[A-Za-z0-9]{30,}\b",
        r"\bnpm_[A-Za-z0-9]{36}\b",
        r"\bSG\.[A-Za-z0-9_\-]{16,}\.[A-Za-z0-9_\-]{16,}",
        r"\bSK[0-9a-fA-F]{32}\b",
        r"\beyJ[A-Za-z0-9_\-]{10,}\.eyJ[A-Za-z0-9_\-]{10,}\.[A-Za-z0-9_\-]{10,}",
    ]
    .iter()
    .map(|p| Regex::new(p).unwrap())
    .collect()
});

/// `scheme://user:password@host`: group 2 is the password.
static URL_CREDENTIALS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([A-Za-z][A-Za-z0-9+.\-]*://[^/\s:@]+:)([^@\s/]+)(@)").unwrap()
});

/// Connection-string fields (`;Password=…`, `AccountKey=…`): group 3 is the value. `Password`
/// and `Pwd` count only after a `;`, so code such as `self.password = password` is left to the
/// assignment rule.
static CONNECTION_FIELDS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)(;\s*(?:Password|Pwd)|\b(?:AccountKey|SharedAccessSignature|SharedAccessKey))(\s*=\s*)([^;"'\s<]+)"#,
    )
    .unwrap()
});

/// `<key>value</key>`: groups 1 and 3 are the tags, group 2 the value.
static XML_ELEMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<([A-Za-z0-9_.:\-]+)>([^<\n]+)</([A-Za-z0-9_.:\-]+)>").unwrap());

/// `key = value`, `key: value`, `"key": "value"`, `export KEY=value`, `key := value`.
static ASSIGNMENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?P<key>[A-Za-z_][A-Za-z0-9_.\-]*)(?P<sep>["']?[ \t]*(?::=|=>|=|:)[ \t]*)(?P<val>"[^"\n]*"|'[^'\n]*'|[^\s,;}\]#]+)"#,
    )
    .unwrap()
});

/// Words that make a key name secret on their own.
const SECRET_WORDS: &[&str] = &[
    "password", "passwd", "pwd", "pass", "secret", "token", "apikey", "credential", "credentials",
    "auth", "dsn", "passphrase", "privatekey", "accesskey", "secretkey", "clientsecret",
];

/// Word suffixes that make a key name secret (`dbpassword`, `accesstoken`).
const SECRET_SUFFIXES: &[&str] = &[
    "password", "passwd", "secret", "token", "apikey", "privatekey", "accesskey", "secretkey",
    "passphrase", "credential", "credentials",
];

/// Two consecutive words that make a key name secret (`api_key`, `client_secret`).
const SECRET_PAIRS: &[(&str, &str)] = &[
    ("api", "key"),
    ("access", "key"),
    ("private", "key"),
    ("secret", "key"),
    ("client", "secret"),
    ("connection", "string"),
    ("signing", "key"),
    ("encryption", "key"),
    ("master", "key"),
];

/// Key words whose values are public checksums or identifiers, never scored by entropy.
const CHECKSUM_WORDS: &[&str] =
    &["hash", "checksum", "integrity", "digest", "sha", "sha1", "sha256", "sha512", "commit", "revision", "rev", "etag", "id", "uuid", "guid"];

/// Type names that follow `name:` in code (`token: String`), not values.
const TYPE_NAMES: &[&str] = &[
    "str", "string", "int", "integer", "bool", "boolean", "number", "any", "bytes", "float", "double",
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize", "isize", "f32", "f64", "char",
    "object", "none", "optional", "option", "vec", "list", "dict",
];

/// Detects and replaces every secret value in `text`. Idempotent; a text with no secret is
/// returned unchanged with `secrets_found == 0`.
pub fn redact_secrets(text: &str) -> Redaction {
    let mut found = 0usize;
    let mut out = redact_private_keys(text, &mut found);

    for re in KNOWN_TOKENS.iter() {
        out = replace_counted(&out, re, &mut found, |_| SECRET_PLACEHOLDER.to_string());
    }

    out = replace_counted_opt(&out, &URL_CREDENTIALS, &mut found, |c| {
        let value = &c[2];
        (!is_placeholder(value) && !is_reference(value))
            .then(|| format!("{}{}{}", &c[1], SECRET_PLACEHOLDER, &c[3]))
    });

    out = replace_counted_opt(&out, &CONNECTION_FIELDS, &mut found, |c| {
        let value = &c[3];
        (!is_placeholder(value) && !is_reference(value))
            .then(|| format!("{}{}{}", &c[1], &c[2], SECRET_PLACEHOLDER))
    });

    out = replace_counted_opt(&out, &XML_ELEMENT, &mut found, |c| {
        let value = c[2].trim();
        (c[1] == c[3] && is_secret_key(&c[1]) && is_secret_value(value))
            .then(|| format!("<{}>{}</{}>", &c[1], SECRET_PLACEHOLDER, &c[3]))
    });

    out = replace_counted_opt(&out, &ASSIGNMENT, &mut found, |c| {
        let key = &c["key"];
        let sep = &c["sep"];
        let raw = &c["val"];
        let (quote, value) = split_quotes(raw);
        let redact = if is_secret_key(key) {
            is_secret_value(value) && !(quote.is_empty() && looks_like_code(value))
        } else {
            entropy_applies(key, sep, quote) && is_high_entropy(value)
        };
        redact.then(|| format!("{key}{sep}{quote}{SECRET_PLACEHOLDER}{quote}"))
    });

    Redaction { text: out, secrets_found: found }
}

/// Replaces each private key block with one placeholder line, keeping its line count. A block
/// without its END line (a truncated file) is redacted up to the end of the text.
fn redact_private_keys(text: &str, found: &mut usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(begin) = PEM_BEGIN.find(rest) {
        out.push_str(&rest[..begin.start()]);
        let after = &rest[begin.start()..];
        let block_len = PEM_END.find(after).map(|end| end.end()).unwrap_or(after.len());
        let newlines = after[..block_len].matches('\n').count();
        out.push_str(SECRET_PLACEHOLDER);
        out.push_str(&"\n".repeat(newlines));
        *found += 1;
        rest = &after[block_len..];
    }
    out.push_str(rest);
    out
}

fn replace_counted(text: &str, re: &Regex, found: &mut usize, f: impl Fn(&Captures) -> String) -> String {
    replace_counted_opt(text, re, found, |c| Some(f(c)))
}

/// Replaces matches for which `f` returns a replacement and counts them; other matches stay.
fn replace_counted_opt(
    text: &str,
    re: &Regex,
    found: &mut usize,
    f: impl Fn(&Captures) -> Option<String>,
) -> String {
    let mut count = 0usize;
    let out = re
        .replace_all(text, |c: &Captures| match f(c) {
            Some(replacement) => {
                count += 1;
                replacement
            }
            None => c[0].to_string(),
        })
        .into_owned();
    *found += count;
    out
}

fn split_quotes(raw: &str) -> (&str, &str) {
    for q in ["\"", "'"] {
        if raw.len() >= 2 && raw.starts_with(q) && raw.ends_with(q) {
            return (q, &raw[1..raw.len() - 1]);
        }
    }
    ("", raw)
}

/// Splits a key name into lowercase words on separators and camelCase boundaries.
fn key_words(key: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_lower = false;
    for ch in key.chars() {
        if !ch.is_ascii_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            prev_lower = false;
            continue;
        }
        if ch.is_ascii_uppercase() && prev_lower && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        prev_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        current.push(ch.to_ascii_lowercase());
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// Whether a key name says its value is a secret (`DB_PASSWORD`, `apiKey`, `client-secret`).
fn is_secret_key(key: &str) -> bool {
    let words = key_words(key);
    words.iter().any(|w| {
        SECRET_WORDS.contains(&w.as_str())
            || SECRET_SUFFIXES.iter().any(|s| w.len() > s.len() && w.ends_with(s))
            || w.starts_with("password")
            || w.starts_with("passwd")
    }) || words
        .windows(2)
        .any(|pair| SECRET_PAIRS.iter().any(|(a, b)| pair[0] == *a && pair[1] == *b))
}

fn is_placeholder(value: &str) -> bool {
    value.starts_with("[SECRET")
}

/// Values that name where a secret comes from instead of holding it.
fn is_reference(value: &str) -> bool {
    let v = value.trim();
    let lower = v.to_ascii_lowercase();
    v.starts_with('$')
        || v.starts_with("{{")
        || (v.starts_with('%') && v.ends_with('%') && v.len() > 2)
        || (v.starts_with('<') && v.ends_with('>'))
        || lower.starts_with("var.")
        || lower.starts_with("local.")
        || lower.starts_with("data.")
        || lower.contains("process.env")
        || lower.contains("os.environ")
        || lower.contains("getenv")
        || lower.contains("env(")
        || lower.contains("secretkeyref")
}

/// Whether a value found under a secret key name is worth redacting.
fn is_secret_value(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() || is_placeholder(v) || is_reference(v) {
        return false;
    }
    let lower = v.to_ascii_lowercase();
    if matches!(lower.as_str(), "true" | "false" | "null" | "nil" | "none" | "yes" | "no" | "on" | "off" | "required" | "optional") {
        return false;
    }
    !(v.len() < 6 && v.chars().all(|c| c.is_ascii_digit()))
}

/// Unquoted values in code that are expressions, types or variables rather than literals.
fn looks_like_code(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    if value.contains('(') || value.contains(')') || value.contains('<') || value.contains("::") || value.starts_with('&') {
        return true;
    }
    if TYPE_NAMES.contains(&lower.trim_end_matches('?').trim_end_matches('|')) {
        return true;
    }
    let identifier = value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.');
    identifier
        && (value.split('.').filter(|p| !p.is_empty()).count() > 1 || is_secret_key(value))
}

/// Entropy is scored only on assignment values: `KEY=value`, or `key: value` with a space after
/// the colon (YAML), never on free text, `h1:` checksums or keys that name a checksum or id.
fn entropy_applies(key: &str, sep: &str, quote: &str) -> bool {
    if key_words(key).iter().any(|w| CHECKSUM_WORDS.contains(&w.as_str())) {
        return false;
    }
    let sep_core = sep.trim_start_matches(['"', '\'']);
    sep_core.contains('=') || (sep_core.trim_start().starts_with(':') && (sep_core.ends_with(' ') || sep_core.ends_with('\t') || !quote.is_empty()))
}

/// A random-looking value: 20+ characters from the base64 or hex alphabets, with letters and
/// digits, and Shannon entropy of at least 3.5 bits per character (hex) or 4.0 (base64).
fn is_high_entropy(value: &str) -> bool {
    if value.len() < 20 || is_placeholder(value) || is_reference(value) {
        return false;
    }
    if !value.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '_' | '-')) {
        return false;
    }
    if !value.chars().any(|c| c.is_ascii_digit()) || !value.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    let lower = value.to_ascii_lowercase();
    if ["sha1-", "sha256-", "sha384-", "sha512-"].iter().any(|p| lower.starts_with(p)) {
        return false;
    }
    let threshold = if value.chars().all(|c| c.is_ascii_hexdigit()) { 3.5 } else { 4.0 };
    shannon_entropy(value) >= threshold
}

fn shannon_entropy(value: &str) -> f64 {
    let mut counts = [0usize; 128];
    for b in value.bytes() {
        counts[(b & 0x7f) as usize] += 1;
    }
    let len = value.len() as f64;
    counts
        .iter()
        .filter(|&&n| n > 0)
        .map(|&n| {
            let p = n as f64 / len;
            -p * p.log2()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Token-shaped sample built at run time, so no complete credential literal is committed.
    fn fake(prefix: &str, len: usize, seed: u64) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        let mut state = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let body: String = (0..len)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                ALPHABET[((state >> 33) as usize) % ALPHABET.len()] as char
            })
            .collect();
        format!("{prefix}{body}")
    }

    fn fake_hex(len: usize, seed: u64) -> String {
        let mut state = seed.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
        (0..len)
            .map(|_| {
                state = state.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
                char::from_digit(((state >> 33) % 16) as u32, 16).unwrap()
            })
            .collect()
    }

    fn assert_redacted(input: &str, secret: &str) -> Redaction {
        let r = redact_secrets(input);
        assert!(!r.text.contains(secret), "secret kept in: {}", r.text);
        assert!(r.text.contains(SECRET_PLACEHOLDER), "no placeholder in: {}", r.text);
        assert!(r.secrets_found >= 1);
        r
    }

    fn assert_kept(input: &str) {
        let r = redact_secrets(input);
        assert_eq!(r.text, input);
        assert_eq!(r.secrets_found, 0);
    }

    #[test]
    fn private_key_block_becomes_one_line_and_keeps_the_line_count() {
        let body = fake("MIIEvQIBADANBgkqhkiG9w0BAQEFAASC", 30, 1);
        let begin = ["-----BEGIN", " RSA PRIVATE KEY-----"].concat();
        let end = ["-----END", " RSA PRIVATE KEY-----"].concat();
        let input = format!("before\n{begin}\n{body}\n{body}\n{end}\nafter\n");
        let r = assert_redacted(&input, &body);
        assert_eq!(r.secrets_found, 1);
        assert_eq!(r.text.lines().count(), input.lines().count());
        assert!(r.text.starts_with("before\n[SECRET REDACTED]\n"));
        assert!(r.text.ends_with("after\n"));
    }

    #[test]
    fn openssh_and_truncated_private_keys_are_redacted() {
        let body = fake("b3BlbnNzaC1rZXktdjEAAAAA", 40, 2);
        let openssh = format!("{}\n{body}\n{}", ["-----BEGIN", " OPENSSH PRIVATE KEY-----"].concat(), ["-----END", " OPENSSH PRIVATE KEY-----"].concat());
        assert_redacted(&openssh, &body);
        let truncated = format!("{}\n{body}\n", ["-----BEGIN", " PRIVATE KEY-----"].concat());
        assert_redacted(&truncated, &body);
    }

    #[test]
    fn public_keys_and_certificates_are_kept() {
        let body = fake("MIIBIjANBgkqhkiG9w0BAQEFAAOC", 30, 3);
        assert_kept(&format!("-----BEGIN PUBLIC KEY-----\n{body}\n-----END PUBLIC KEY-----\n"));
        assert_kept(&format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n"));
    }

    #[test]
    fn known_provider_tokens_are_redacted_anywhere_in_the_text() {
        let samples = [
            fake(&["AK", "IA"].concat(), 0, 0) + &fake_hex(16, 4).to_uppercase(),
            fake(&["gh", "p_"].concat(), 36, 5),
            fake(&["github", "_pat_"].concat(), 40, 6),
            fake(&["gl", "pat-"].concat(), 20, 7),
            fake(&["xo", "xb-"].concat(), 24, 8),
            fake(&["sk", "_live_"].concat(), 24, 9),
            fake(&["rk", "_test_"].concat(), 24, 10),
            fake(&["AI", "za"].concat(), 35, 11),
            fake(&["sk", "-ant-"].concat(), 40, 12),
            fake(&["sk", "-proj-"].concat(), 48, 13),
            fake(&["sk", "-"].concat(), 40, 14),
            fake(&["hf", "_"].concat(), 34, 15),
            fake(&["np", "m_"].concat(), 36, 16),
            format!("{}.{}", fake(&["S", "G."].concat(), 22, 17), fake("", 43, 18)),
            format!("{}{}", ["S", "K"].concat(), fake_hex(32, 19)),
            format!("{}.{}.{}", fake(&["ey", "J"].concat(), 20, 20), fake(&["ey", "J"].concat(), 30, 21), fake("", 30, 22)),
        ];
        for token in &samples {
            assert_redacted(&format!("The deploy used {token} yesterday."), token);
        }
    }

    #[test]
    fn slack_webhook_urls_are_redacted() {
        let url = format!("https://hooks.slack.com/services/{}/{}", fake("T", 8, 23), fake("", 24, 24));
        assert_redacted(&format!("webhook {url} here"), &url);
    }

    #[test]
    fn passwords_in_urls_and_connection_strings_are_redacted() {
        let password = fake("", 14, 25);
        let r = assert_redacted(&format!("postgres://app:{password}@db.internal:5432/app"), &password);
        assert_eq!(r.text, "postgres://app:[SECRET REDACTED]@db.internal:5432/app");
        let r = assert_redacted(&format!("Server=db;User Id=sa;Password={password};Encrypt=true"), &password);
        assert!(r.text.contains("User Id=sa;Password=[SECRET REDACTED];Encrypt=true"));
        let key = fake("", 64, 26);
        assert_redacted(&format!("DefaultEndpointsProtocol=https;AccountName=acct;AccountKey={key};"), &key);
    }

    #[test]
    fn url_references_are_kept() {
        assert_kept("postgres://app:${DB_PASSWORD}@db.internal/app");
        assert_kept("see https://docs.example.com/guide:intro for details");
    }

    #[test]
    fn secret_named_assignments_are_redacted_in_every_syntax() {
        let v = fake("", 12, 27);
        let cases = [
            (format!("DB_PASSWORD={v}"), "DB_PASSWORD=[SECRET REDACTED]".to_string()),
            (format!("export API_KEY={v}"), "export API_KEY=[SECRET REDACTED]".to_string()),
            (format!("client_secret: {v}"), "client_secret: [SECRET REDACTED]".to_string()),
            (format!("{{\"apiKey\": \"{v}\", \"region\": \"eu\"}}"), "{\"apiKey\": \"[SECRET REDACTED]\", \"region\": \"eu\"}".to_string()),
            (format!("access_token = \"{v}\""), "access_token = \"[SECRET REDACTED]\"".to_string()),
            (format!("db.password={v}"), "db.password=[SECRET REDACTED]".to_string()),
            (format!("const authToken = '{v}';"), "const authToken = '[SECRET REDACTED]';".to_string()),
            (format!("<password>{v}</password>"), "<password>[SECRET REDACTED]</password>".to_string()),
            (format!("private_key_passphrase := \"{v}\""), "private_key_passphrase := \"[SECRET REDACTED]\"".to_string()),
        ];
        for (input, expected) in &cases {
            let r = assert_redacted(input, &v);
            assert_eq!(&r.text, expected);
        }
    }

    #[test]
    fn short_and_plain_secret_values_are_redacted() {
        let r = redact_secrets("password: hunter2\n");
        assert_eq!(r.text, "password: [SECRET REDACTED]\n");
    }

    #[test]
    fn references_types_and_flags_under_secret_names_are_kept() {
        for line in [
            "DB_PASSWORD=${DB_PASSWORD}",
            "api_key = var.api_key",
            "token: \"{{ .Values.token }}\"",
            "const apiKey = process.env.API_KEY;",
            "password = os.environ[\"PASSWORD\"]",
            "secret = get_secret(\"name\")",
            "    token: String,",
            "def login(password: str) -> bool:",
            "fn sign(api_key: &str) {}",
            "self.password = password",
            "password_required: true",
            "TOKEN_TTL=3600",
            "PASSWORD=",
            "client_secret: <your-client-secret>",
            "max_tokens: 1000",
            "author: Jane Doe",
            "tokenizer: bert-base-multilingual",
        ] {
            assert_kept(line);
        }
    }

    #[test]
    fn high_entropy_assignment_values_are_redacted_whatever_the_key() {
        let b64 = fake("", 40, 28);
        assert_redacted(&format!("STRIPE_WEBHOOK={b64}"), &b64);
        assert_redacted(&format!("upstream: {b64}"), &b64);
        let hex = fake_hex(40, 29);
        assert_redacted(&format!("SESSION_SALT={hex}"), &hex);
    }

    #[test]
    fn checksums_hashes_and_free_text_are_not_scored_by_entropy() {
        let hex = fake_hex(40, 30);
        assert_kept(&format!("Fixed in commit {hex} on main."));
        assert_kept(&format!("commit: {hex}"));
        let b64 = fake("", 43, 31);
        assert_kept(&format!("golang.org/x/text v0.14.0 h1:{b64}="));
        assert_kept(&format!("\"integrity\": \"sha512-{b64}==\""));
        assert_kept("APP_NAME=billing-service-production-eu");
        assert_kept("id: 3f2a9c1e-7b4d-4e8a-9f1c-2d3e4f5a6b7c");
    }

    #[test]
    fn rest_of_the_file_and_key_names_are_kept() {
        let v = fake("", 20, 32);
        let input = format!("# Settings\nAPP_NAME=billing\nOPENAI_API_KEY={v}\nPORT=8080\n");
        let r = assert_redacted(&input, &v);
        assert_eq!(r.text, "# Settings\nAPP_NAME=billing\nOPENAI_API_KEY=[SECRET REDACTED]\nPORT=8080\n");
        assert_eq!(r.secrets_found, 1);
    }

    #[test]
    fn redaction_is_idempotent() {
        let v = fake(&["sk", "-proj-"].concat(), 48, 33);
        let once = redact_secrets(&format!("OPENAI_API_KEY={v}\npassword: \"{v}\"\n"));
        let twice = redact_secrets(&once.text);
        assert_eq!(twice.text, once.text);
        assert_eq!(twice.secrets_found, 0);
    }

    #[test]
    fn text_without_secrets_is_unchanged() {
        assert_kept("# Readme\nThis project indexes documents.\nRun `cargo test` to check it.\n");
        assert_kept("");
    }

    #[test]
    fn key_words_split_on_separators_and_camel_case() {
        assert_eq!(key_words("clientSecretKey"), vec!["client", "secret", "key"]);
        assert_eq!(key_words("DB_PASSWORD"), vec!["db", "password"]);
        assert_eq!(key_words("x-api-key"), vec!["x", "api", "key"]);
    }
}
