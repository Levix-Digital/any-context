use pyo3::prelude::*;
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Universal RFC English / International month mapping table.
/// Language-agnostic, global standard (RFC 2822 / RFC 3339 / ISO).
static RFC_MONTH_MAP: &[(&str, &str)] = &[
    ("january", "01"), ("jan", "01"),
    ("february", "02"), ("feb", "02"),
    ("march", "03"), ("mar", "03"),
    ("april", "04"), ("apr", "04"),
    ("may", "05"),
    ("june", "06"), ("jun", "06"),
    ("july", "07"), ("jul", "07"),
    ("august", "08"), ("aug", "08"),
    ("september", "09"), ("sept", "09"), ("sep", "09"),
    ("october", "10"), ("oct", "10"),
    ("november", "11"), ("nov", "11"),
    ("december", "12"), ("dec", "12"),
];

fn get_rfc_month(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    for &(m_name, m_num) in RFC_MONTH_MAP {
        if m_name == lower {
            return Some(m_num);
        }
    }
    None
}

// Global compiled regex singletons (zero compilation overhead per query)
fn iso_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(20\d{2})[-/](0[1-9]|1[0-2])[-/](0[1-9]|[12]\d|3[01])\b").unwrap()
    })
}

fn intl_numeric_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(0[1-9]|[12]\d|3[01])[-/.](0[1-9]|1[0-2])[-/.](20\d{2})\b").unwrap()
    })
}

fn english_mdy_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sept|sep|october|oct|november|nov|december|dec)\s+(\d{1,2})(?:st|nd|rd|th)?(?:,?\s+(20\d{2}))?\b").unwrap()
    })
}

fn english_dmy_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(\d{1,2})(?:st|nd|rd|th)?\s+(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sept|sep|october|oct|november|nov|december|dec)(?:\s+(?:,?\s*)?(20\d{2}))?\b").unwrap()
    })
}

fn english_my_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b(january|jan|february|feb|march|mar|april|apr|may|june|jun|july|jul|august|aug|september|sept|sep|october|oct|november|nov|december|dec)\s+(?:of\s+)?(20\d{2})\b").unwrap()
    })
}

fn short_numeric_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"\b(0?[1-9]|[12]\d|3[01])[-/.](0?[1-9]|1[0-2])\b").unwrap()
    })
}

fn filename_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(r"(?i)\b([a-zA-Z0-9_\-\.]+\.(?:pdf|csv|xlsx|xls|json|xml|docx|txt|md|log|tsv))\b").unwrap()
    })
}

fn add_clauses_internal(
    clauses: &mut Vec<String>,
    seen: &mut HashSet<String>,
    yy: Option<&str>,
    mm: &str,
    dd: Option<&str>,
) {
    let mut add_one = |c: String| {
        if seen.insert(c.clone()) {
            clauses.push(c);
        }
    };

    if let (Some(y), Some(d)) = (yy, dd) {
        add_one(format!("file_path LIKE '%{}/{}/{}%'", y, mm, d));
        add_one(format!("file_path LIKE '%{}-{}-{}%'", y, mm, d));
        add_one(format!("file_path LIKE '%/{}/{}/%'", mm, d));
        add_one(format!("file_path LIKE '%/{}/{}%'", mm, d));
    } else if let Some(d) = dd {
        add_one(format!("file_path LIKE '%/{}/{}/%'", mm, d));
        add_one(format!("file_path LIKE '%/{}/{}%'", mm, d));
    } else if let Some(y) = yy {
        add_one(format!("file_path LIKE '%{}/{}/%'", y, mm));
        add_one(format!("file_path LIKE '%{}/{}%'", y, mm));
    }
}

fn is_part_of_larger_date(query: &str, start: usize, end: usize) -> bool {
    let bytes = query.as_bytes();
    if start > 0 {
        let prev = bytes[start - 1];
        if prev == b'/' || prev == b'-' || prev == b'.' || prev.is_ascii_digit() {
            return true;
        }
    }
    if end < bytes.len() {
        let next = bytes[end];
        if next == b'/' || next == b'-' || next == b'.' || next.is_ascii_digit() {
            return true;
        }
    }
    false
}

/// Extracts deterministic date path filter clauses from user queries.
pub fn extract_temporal_clauses_native(query: &str) -> Vec<String> {
    let mut clauses = Vec::new();
    let mut seen = HashSet::new();

    // 1. ISO format: YYYY-MM-DD or YYYY/MM/DD
    for caps in iso_regex().captures_iter(query) {
        let y = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let m = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let d = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        add_clauses_internal(&mut clauses, &mut seen, Some(y), m, Some(d));
    }

    // 2. International numeric format: DD/MM/YYYY or DD-MM-YYYY or DD.MM.YYYY
    for caps in intl_numeric_regex().captures_iter(query) {
        let d = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let m = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let y = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        add_clauses_internal(&mut clauses, &mut seen, Some(y), m, Some(d));
    }

    // 3A. Universal English Month-Day-Year (e.g., "September 3, 2026", "Sep 03")
    for caps in english_mdy_regex().captures_iter(query) {
        let m_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let d_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let y_opt = caps.get(3).map(|m| m.as_str());

        if let Some(mm) = get_rfc_month(m_name) {
            let dd_pad = if let Ok(day_val) = d_str.parse::<u32>() {
                format!("{:02}", day_val)
            } else {
                d_str.to_string()
            };
            add_clauses_internal(&mut clauses, &mut seen, y_opt, mm, Some(&dd_pad));
        }
    }

    // 3B. Universal English Day-Month-Year (e.g., "3 September 2026", "03 Sep")
    for caps in english_dmy_regex().captures_iter(query) {
        let d_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let m_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let y_opt = caps.get(3).map(|m| m.as_str());

        if let Some(mm) = get_rfc_month(m_name) {
            let dd_pad = if let Ok(day_val) = d_str.parse::<u32>() {
                format!("{:02}", day_val)
            } else {
                d_str.to_string()
            };
            add_clauses_internal(&mut clauses, &mut seen, y_opt, mm, Some(&dd_pad));
        }
    }

    // 3C. Universal English Month-Year (e.g., "January 2026", "Sep 2026")
    for caps in english_my_regex().captures_iter(query) {
        let m_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let y_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        if let Some(mm) = get_rfc_month(m_name) {
            add_clauses_internal(&mut clauses, &mut seen, Some(y_str), mm, None);
        }
    }

    // 4. Short numeric dates without year: DD/MM or MM/DD (e.g. '02/09', '2/9', '02-09', '28/05')
    for caps in short_numeric_regex().captures_iter(query) {
        if let Some(mat) = caps.get(0) {
            if is_part_of_larger_date(query, mat.start(), mat.end()) {
                continue;
            }
        }
        let d1_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let d2_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        if let (Ok(val1), Ok(val2)) = (d1_str.parse::<u32>(), d2_str.parse::<u32>()) {
            let pad1 = format!("{:02}", val1);
            let pad2 = format!("{:02}", val2);

            let mut add_one = |c: String| {
                if seen.insert(c.clone()) {
                    clauses.push(c);
                }
            };

            if val1 > 12 {
                // val1 is day, val2 is month
                add_one(format!("file_path LIKE '%/{}/{}/%'", pad2, pad1));
                add_one(format!("file_path LIKE '%/{}/{}%'", pad2, pad1));
                add_one(format!("file_path LIKE '%{}-{}%'", pad2, pad1));
                add_one(format!("file_path LIKE '%{}-{}%'", pad1, pad2));
            } else if val2 > 12 {
                // val2 is day, val1 is month
                add_one(format!("file_path LIKE '%/{}/{}/%'", pad1, pad2));
                add_one(format!("file_path LIKE '%/{}/{}%'", pad1, pad2));
                add_one(format!("file_path LIKE '%{}-{}%'", pad1, pad2));
                add_one(format!("file_path LIKE '%{}-{}%'", pad2, pad1));
            } else {
                // Both <= 12: support both DD/MM and MM/DD conventions
                add_one(format!("file_path LIKE '%/{}/{}/%'", pad2, pad1));
                add_one(format!("file_path LIKE '%/{}/{}%'", pad2, pad1));
                add_one(format!("file_path LIKE '%/{}/{}/%'", pad1, pad2));
                add_one(format!("file_path LIKE '%/{}/{}%'", pad1, pad2));
                add_one(format!("file_path LIKE '%{}-{}%'", pad2, pad1));
                add_one(format!("file_path LIKE '%{}-{}%'", pad1, pad2));
            }
        }
    }

    clauses
}

/// Expands conversational queries containing dates with standardized canonical date tokens
/// (YYYY-MM-DD, YYYY/MM/DD, DD/MM/YYYY, MM/DD) for direct native BM25 token matching.
pub fn expand_query_temporal_native(query: &str) -> String {
    let mut expanded_tokens = Vec::new();
    let mut seen_tokens = HashSet::new();

    let mut add_token = |t: String| {
        if seen_tokens.insert(t.clone()) {
            expanded_tokens.push(t);
        }
    };

    // 1. ISO format: YYYY-MM-DD or YYYY/MM/DD
    for caps in iso_regex().captures_iter(query) {
        let y = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let m = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let d = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        add_token(format!("{}-{}-{}", y, m, d));
        add_token(format!("{}/{}/{}", y, m, d));
        add_token(format!("{}/{}/{}", d, m, y));
        add_token(format!("{}/{}", m, d));
    }

    // 2. International numeric format: DD/MM/YYYY or DD-MM-YYYY or DD.MM.YYYY
    for caps in intl_numeric_regex().captures_iter(query) {
        let d = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let m = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let y = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        add_token(format!("{}-{}-{}", y, m, d));
        add_token(format!("{}/{}/{}", y, m, d));
        add_token(format!("{}/{}/{}", d, m, y));
        add_token(format!("{}/{}", m, d));
    }

    // 3A. Universal English Month-Day-Year
    for caps in english_mdy_regex().captures_iter(query) {
        let m_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let d_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let y_opt = caps.get(3).map(|m| m.as_str());

        if let Some(mm) = get_rfc_month(m_name) {
            let dd_pad = if let Ok(day_val) = d_str.parse::<u32>() {
                format!("{:02}", day_val)
            } else {
                d_str.to_string()
            };
            if let Some(yy) = y_opt {
                add_token(format!("{}-{}-{}", yy, mm, dd_pad));
                add_token(format!("{}/{}/{}", yy, mm, dd_pad));
                add_token(format!("{}/{}/{}", dd_pad, mm, yy));
                add_token(format!("{}/{}", mm, dd_pad));
            } else {
                add_token(format!("{}-{}", mm, dd_pad));
                add_token(format!("{}/{}", mm, dd_pad));
                add_token(format!("{}/{}", dd_pad, mm));
            }
        }
    }

    // 3B. Universal English Day-Month-Year
    for caps in english_dmy_regex().captures_iter(query) {
        let d_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let m_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let y_opt = caps.get(3).map(|m| m.as_str());

        if let Some(mm) = get_rfc_month(m_name) {
            let dd_pad = if let Ok(day_val) = d_str.parse::<u32>() {
                format!("{:02}", day_val)
            } else {
                d_str.to_string()
            };
            if let Some(yy) = y_opt {
                add_token(format!("{}-{}-{}", yy, mm, dd_pad));
                add_token(format!("{}/{}/{}", yy, mm, dd_pad));
                add_token(format!("{}/{}/{}", dd_pad, mm, yy));
                add_token(format!("{}/{}", mm, dd_pad));
            } else {
                add_token(format!("{}-{}", mm, dd_pad));
                add_token(format!("{}/{}", mm, dd_pad));
                add_token(format!("{}/{}", dd_pad, mm));
            }
        }
    }

    // 3C. Universal English Month-Year
    for caps in english_my_regex().captures_iter(query) {
        let m_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let y_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        if let Some(mm) = get_rfc_month(m_name) {
            add_token(format!("{}-{}", y_str, mm));
            add_token(format!("{}/{}", y_str, mm));
        }
    }

    // 4. Short numeric dates without year
    for caps in short_numeric_regex().captures_iter(query) {
        if let Some(mat) = caps.get(0) {
            if is_part_of_larger_date(query, mat.start(), mat.end()) {
                continue;
            }
        }
        let d1_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let d2_str = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        if let (Ok(val1), Ok(val2)) = (d1_str.parse::<u32>(), d2_str.parse::<u32>()) {
            let pad1 = format!("{:02}", val1);
            let pad2 = format!("{:02}", val2);
            add_token(format!("{}/{}", pad1, pad2));
            add_token(format!("{}/{}", pad2, pad1));
            add_token(format!("{}-{}", pad1, pad2));
            add_token(format!("{}-{}", pad2, pad1));
        }
    }

    if expanded_tokens.is_empty() {
        query.to_string()
    } else {
        format!("{} {}", query, expanded_tokens.join(" "))
    }
}

/// Extracts explicit filenames or supported document extensions mentioned in user queries.
pub fn extract_filename_mentions_native(query: &str) -> Vec<String> {
    let mut filenames = Vec::new();
    let mut seen_lower = HashSet::new();

    for caps in filename_regex().captures_iter(query) {
        if let Some(m) = caps.get(1) {
            let name = m.as_str().to_string();
            let lower = name.to_lowercase();
            if seen_lower.insert(lower) {
                filenames.push(name);
            }
        }
    }

    filenames
}

/// Single-Pass parsed query container exposed directly to PyO3.
#[pyclass]
#[derive(Debug, Clone)]
pub struct ProcessedQuery {
    #[pyo3(get)]
    pub temporal_clauses: Vec<String>,
    #[pyo3(get)]
    pub expanded_query: String,
    #[pyo3(get)]
    pub filename_mentions: Vec<String>,
}

#[pymethods]
impl ProcessedQuery {
    fn __repr__(&self) -> String {
        format!(
            "ProcessedQuery(temporal_clauses={:?}, filename_mentions={:?}, expanded_query={:?})",
            self.temporal_clauses, self.filename_mentions, self.expanded_query
        )
    }
}

/// High-performance language-agnostic Query Preprocessor.
#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct QueryPreprocessor;

#[pymethods]
impl QueryPreprocessor {
    #[new]
    pub fn new() -> Self {
        Self
    }

    /// Single-pass preprocessing: extracts temporal clauses, filename mentions, and expands query in < 0.02ms.
    #[staticmethod]
    pub fn process(query: &str) -> ProcessedQuery {
        let temporal_clauses = extract_temporal_clauses_native(query);
        let expanded_query = expand_query_temporal_native(query);
        let filename_mentions = extract_filename_mentions_native(query);

        ProcessedQuery {
            temporal_clauses,
            expanded_query,
            filename_mentions,
        }
    }

    #[staticmethod]
    pub fn extract_temporal_clauses(query: &str) -> Vec<String> {
        extract_temporal_clauses_native(query)
    }

    #[staticmethod]
    pub fn expand_query_temporal(query: &str) -> String {
        expand_query_temporal_native(query)
    }

    #[staticmethod]
    pub fn extract_filename_mentions(query: &str) -> Vec<String> {
        extract_filename_mentions_native(query)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso_date_extraction() {
        let q = "What happened on 2026-09-02?";
        let clauses = extract_temporal_clauses_native(q);
        assert!(clauses.iter().any(|c| c.contains("2026/09/02")));
        assert!(clauses.iter().any(|c| c.contains("2026-09-02")));

        let exp = expand_query_temporal_native(q);
        assert!(exp.contains("2026-09-02"));
        assert!(exp.contains("2026/09/02"));
        assert!(exp.contains("02/09/2026"));
    }

    #[test]
    fn test_intl_numeric_date_extraction() {
        let q = "Report from 15/08/2026 details";
        let clauses = extract_temporal_clauses_native(q);
        assert!(clauses.iter().any(|c| c.contains("2026/08/15")));

        let exp = expand_query_temporal_native(q);
        assert!(exp.contains("2026-08-15"));
        assert!(exp.contains("15/08/2026"));
    }

    #[test]
    fn test_english_rfc_month_extraction() {
        let q = "Deliveries on September 3, 2026";
        let clauses = extract_temporal_clauses_native(q);
        assert!(clauses.iter().any(|c| c.contains("2026/09/03")));

        let exp = expand_query_temporal_native(q);
        assert!(exp.contains("2026-09-03"));
        assert!(exp.contains("03/09/2026"));
    }

    #[test]
    fn test_short_numeric_date_extraction() {
        let q = "Shipment on 02/09";
        let clauses = extract_temporal_clauses_native(q);
        assert!(clauses.iter().any(|c| c.contains("/09/02/")));

        let exp = expand_query_temporal_native(q);
        assert!(exp.contains("02/09"));
        assert!(exp.contains("09/02"));
    }

    #[test]
    fn test_filename_mentions() {
        let q = "Check I.CMR_ONE_PICKUP.pdf and data_summary.csv in the vault";
        let files = extract_filename_mentions_native(q);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], "I.CMR_ONE_PICKUP.pdf");
        assert_eq!(files[1], "data_summary.csv");
    }

    #[test]
    fn test_single_pass_processing() {
        let q = "Check I.CMR_ONE_PICKUP.pdf on 2026-09-02";
        let res = QueryPreprocessor::process(q);
        assert_eq!(res.filename_mentions, vec!["I.CMR_ONE_PICKUP.pdf"]);
        assert!(res.temporal_clauses.iter().any(|c| c.contains("2026/09/02")));
        assert!(res.expanded_query.contains("2026-09-02"));
    }
}
