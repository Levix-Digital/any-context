//! Prompt safety primitives shared by every prompt that carries retrieved text (ADR-0136, ADR-0148).
//!
//! Retrieved text, metadata and queries are data. These helpers keep them from opening or closing
//! the reserved envelope tags, from forging attributes and from expanding template placeholders.
//! The canonical context formatter of the core and the Deep Search prompts both use them.

use regex::Regex;
use std::sync::OnceLock;

/// Tag names of the prompt envelopes. Retrieved text and queries must never open or close them,
/// so any tag added to an envelope must be listed here.
pub const RESERVED_PROMPT_TAGS: &[&str] = &[
    "retrieved_context",
    "chunk",
    "system_instructions",
    "user_query",
    "tool_result",
    "query_anchor",
    "live_status",
    "sub_queries",
];

/// Matches the `<` that opens or closes a reserved tag (any case, optional spaces and slash).
fn reserved_tag_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        let pattern = format!(r"(?i)<(\s*/?\s*(?:{})\b)", RESERVED_PROMPT_TAGS.join("|"));
        Regex::new(&pattern).unwrap()
    })
}

/// Replaces the `<` of every reserved envelope tag with `&lt;`, leaving any other markup intact.
pub fn neutralize_reserved_tags(text: &str) -> String {
    reserved_tag_regex().replace_all(text, "&lt;${1}").into_owned()
}

/// Escapes a value for a double-quoted XML attribute and keeps it on a single line.
pub fn escape_xml_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Fills `{name}` placeholders in one left-to-right pass.
///
/// Inserted values are never scanned again, so a value containing `{name}` stays literal.
/// Placeholders without a value, and any other braces, are kept as they are.
pub fn fill_template(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len() + values.iter().map(|(_, v)| v.len()).sum::<usize>());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let value = after
            .find('}')
            .and_then(|close| values.iter().find(|(name, _)| *name == &after[..close]).map(|(_, v)| (close, *v)));
        match value {
            Some((close, v)) => {
                out.push_str(v);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neutralize_reserved_tags_catches_case_spaces_and_slash() {
        let text = "</chunk> </CHUNK > < /user_query> <sub_queries> <Retrieved_Context>";
        assert_eq!(
            neutralize_reserved_tags(text),
            "&lt;/chunk> &lt;/CHUNK > &lt; /user_query> &lt;sub_queries> &lt;Retrieved_Context>"
        );
    }

    #[test]
    fn test_neutralize_reserved_tags_keeps_ordinary_markup() {
        let text = "fn f<T>(v: Vec<T>) {} if a < b && c > d {} <div class=\"x\"></div> <chunks> <chunk_id>";
        assert_eq!(neutralize_reserved_tags(text), text);
    }

    #[test]
    fn test_escape_xml_attr_escapes_and_flattens() {
        assert_eq!(escape_xml_attr("a\"b'c<d>&e\nf\tg"), "a&quot;b&apos;c&lt;d&gt;&amp;e f g");
    }

    #[test]
    fn test_fill_template_is_single_pass() {
        let filled = fill_template("Q: {query}\nC: {chunk}", &[("query", "see {chunk}"), ("chunk", "body {query}")]);
        assert_eq!(filled, "Q: see {chunk}\nC: body {query}");
    }

    #[test]
    fn test_fill_template_keeps_unknown_placeholders_and_braces() {
        let filled = fill_template(r#"{a} ["x", "y"] {unknown} {"k": 1} { open"#, &[("a", "A")]);
        assert_eq!(filled, r#"A ["x", "y"] {unknown} {"k": 1} { open"#);
    }

    #[test]
    fn test_fill_template_replaces_every_occurrence() {
        assert_eq!(fill_template("{a}-{a}", &[("a", "1")]), "1-1");
    }
}
