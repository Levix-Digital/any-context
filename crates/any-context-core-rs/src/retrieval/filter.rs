//! Filter of a retrieval request, applied inside each retriever before its candidate limit
//! (IMP-040, ADR-0155).
//!
//! The workspace, the embedding target and the `Global` rule stay where they are (request and
//! pipeline); this filter narrows a search further: to source scopes, a modification date range
//! and content types. The same rule is rendered twice: as a LanceDB predicate for the dense
//! retriever and as [`RetrievalFilter::matches`] for the lexical one.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Narrowing of one search. The default narrows nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalFilter {
    /// Folders, files or web roots: a chunk passes when its path is one of them or lies under one.
    #[serde(default)]
    pub sources: Vec<String>,
    /// Inclusive lower bound of the file modification time, UTC RFC 3339.
    #[serde(default)]
    pub modified_from: Option<String>,
    /// Exclusive upper bound of the file modification time, UTC RFC 3339.
    #[serde(default)]
    pub modified_to: Option<String>,
    /// Content types a chunk must have (as stored, e.g. `Local Document`).
    #[serde(default)]
    pub content_types: Vec<String>,
}

/// Same form as the stored `last_modified` values (`DateTime<Utc>::to_rfc3339`), so the strings
/// compare in time order.
fn stored_form(at: DateTime<Utc>) -> String {
    at.to_rfc3339()
}

/// Path with `/` separators and no trailing separator.
fn normalize_path(path: &str) -> String {
    let p = path.trim().replace('\\', "/");
    let trimmed = p.trim_end_matches('/');
    if trimmed.is_empty() { p } else { trimmed.to_string() }
}

/// SQL string literal: single quotes doubled.
fn sql_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

impl RetrievalFilter {
    /// Sets the modification date range. `None` leaves that side open.
    pub fn with_modified_range(mut self, from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Self {
        self.modified_from = from.map(stored_form);
        self.modified_to = to.map(stored_form);
        self
    }

    /// Whether the filter narrows anything.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty() && self.modified_from.is_none() && self.modified_to.is_none() && self.content_types.is_empty()
    }

    fn date_bounds(&self) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        let parse = |v: &Option<String>| {
            v.as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc))
        };
        (parse(&self.modified_from), parse(&self.modified_to))
    }

    /// LanceDB (DataFusion SQL) predicate of the filter, or `None` when it narrows nothing.
    /// Paths are matched with either separator, as stored paths keep the platform's.
    pub fn lance_predicate(&self) -> Option<String> {
        let mut clauses = Vec::new();

        let scopes: Vec<String> = self
            .sources
            .iter()
            .map(|s| normalize_path(s))
            .filter(|s| !s.is_empty())
            .flat_map(|slash| {
                let back = slash.replace('/', "\\");
                let mut forms = vec![(slash.clone(), '/')];
                if back != slash {
                    forms.push((back, '\\'));
                }
                forms.into_iter().map(|(form, sep)| {
                    // `starts_with` takes the prefix literally: no LIKE wildcard to escape (LanceDB
                    // accepts no LIKE escape character other than the backslash, a path separator).
                    format!(
                        "file_path = {} OR starts_with(file_path, {})",
                        sql_literal(&form),
                        sql_literal(&format!("{form}{sep}"))
                    )
                })
            })
            .collect();
        if !scopes.is_empty() {
            clauses.push(format!("({})", scopes.join(" OR ")));
        }

        let (from, to) = self.date_bounds();
        if let Some(from) = from {
            clauses.push(format!("last_modified >= {}", sql_literal(&stored_form(from))));
        }
        if let Some(to) = to {
            clauses.push(format!("last_modified < {}", sql_literal(&stored_form(to))));
        }
        // A bound that does not parse matches nothing rather than everything.
        if (self.modified_from.is_some() && from.is_none()) || (self.modified_to.is_some() && to.is_none()) {
            clauses.push("false".to_string());
        }

        if !self.content_types.is_empty() {
            let list = self.content_types.iter().map(|t| sql_literal(t)).collect::<Vec<_>>().join(", ");
            clauses.push(format!("content_type IN ({list})"));
        }

        (!clauses.is_empty()).then(|| clauses.join(" AND "))
    }

    /// Whether a chunk with these fields passes the filter (the lexical retriever's rule, the
    /// same as [`Self::lance_predicate`]).
    pub fn matches(&self, file_path: &str, content_type: &str, last_modified: Option<&str>) -> bool {
        if !self.sources.is_empty() {
            let path = file_path.replace('\\', "/");
            let in_scope = self.sources.iter().map(|s| normalize_path(s)).filter(|s| !s.is_empty()).any(|scope| {
                path == scope || path.strip_prefix(scope.as_str()).is_some_and(|rest| rest.starts_with('/'))
            });
            if !in_scope {
                return false;
            }
        }

        if self.modified_from.is_some() || self.modified_to.is_some() {
            let (from, to) = self.date_bounds();
            if (self.modified_from.is_some() && from.is_none()) || (self.modified_to.is_some() && to.is_none()) {
                return false;
            }
            let Some(at) = last_modified
                .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
                .map(|d| d.with_timezone(&Utc))
            else {
                return false;
            };
            if from.is_some_and(|f| at < f) || to.is_some_and(|t| at >= t) {
                return false;
            }
        }

        if !self.content_types.is_empty() && !self.content_types.iter().any(|t| t == content_type) {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).unwrap()
    }

    fn stamp(y: i32, m: u32, d: u32) -> String {
        at(y, m, d).to_rfc3339()
    }

    #[test]
    fn empty_filter_narrows_nothing() {
        let f = RetrievalFilter::default();
        assert!(f.is_empty());
        assert_eq!(f.lance_predicate(), None);
        assert!(f.matches("/any/path.md", "Local Document", None));
    }

    #[test]
    fn source_scope_matches_the_folder_and_what_is_under_it_only() {
        let f = RetrievalFilter { sources: vec!["/data/docs/".to_string()], ..Default::default() };
        assert!(f.matches("/data/docs", "t", None));
        assert!(f.matches("/data/docs/a.md", "t", None));
        assert!(f.matches("/data/docs/sub/b.md", "t", None));
        assert!(!f.matches("/data/docs-old/a.md", "t", None));
        assert!(!f.matches("/data/other/a.md", "t", None));
    }

    #[test]
    fn source_scope_accepts_either_separator() {
        let f = RetrievalFilter { sources: vec!["C:\\data\\docs".to_string()], ..Default::default() };
        assert!(f.matches("C:\\data\\docs\\a.md", "t", None));
        assert!(f.matches("C:/data/docs/a.md", "t", None));
        let pred = f.lance_predicate().unwrap();
        assert!(pred.contains("file_path = 'C:/data/docs'"), "{pred}");
        assert!(pred.contains("file_path = 'C:\\data\\docs'"), "{pred}");
    }

    #[test]
    fn several_sources_are_alternatives() {
        let f = RetrievalFilter { sources: vec!["/a".to_string(), "/b".to_string()], ..Default::default() };
        assert!(f.matches("/a/x", "t", None));
        assert!(f.matches("/b/y", "t", None));
        assert!(!f.matches("/c/z", "t", None));
    }

    #[test]
    fn values_are_escaped_in_the_predicate() {
        let f = RetrievalFilter {
            sources: vec!["/it's 100%_done!".to_string()],
            content_types: vec!["x' OR '1'='1".to_string()],
            ..Default::default()
        };
        let pred = f.lance_predicate().unwrap();
        assert!(pred.contains("file_path = '/it''s 100%_done!'"), "{pred}");
        assert!(pred.contains("starts_with(file_path, '/it''s 100%_done!/')"), "{pred}");
        assert!(pred.contains("content_type IN ('x'' OR ''1''=''1')"), "{pred}");
        // The literal still matches only itself on the lexical side.
        assert!(f.matches("/it's 100%_done!/a.md", "x' OR '1'='1", None));
        assert!(!f.matches("/it's 100%_done!/a.md", "x", None));
    }

    #[test]
    fn date_range_is_inclusive_below_and_exclusive_above() {
        let f = RetrievalFilter::default().with_modified_range(Some(at(2025, 3, 1)), Some(at(2025, 4, 1)));
        assert!(f.matches("/a", "t", Some(&stamp(2025, 3, 1))));
        assert!(f.matches("/a", "t", Some(&stamp(2025, 3, 31))));
        assert!(!f.matches("/a", "t", Some(&stamp(2025, 4, 1))));
        assert!(!f.matches("/a", "t", Some(&stamp(2025, 2, 28))));
        let pred = f.lance_predicate().unwrap();
        assert_eq!(pred, format!("last_modified >= '{}' AND last_modified < '{}'", stamp(2025, 3, 1), stamp(2025, 4, 1)));
    }

    #[test]
    fn open_date_ranges() {
        let from = RetrievalFilter::default().with_modified_range(Some(at(2025, 3, 1)), None);
        assert!(from.matches("/a", "t", Some(&stamp(2030, 1, 1))));
        assert!(!from.matches("/a", "t", Some(&stamp(2025, 1, 1))));
        let to = RetrievalFilter::default().with_modified_range(None, Some(at(2025, 3, 1)));
        assert!(to.matches("/a", "t", Some(&stamp(2000, 1, 1))));
        assert!(!to.matches("/a", "t", Some(&stamp(2025, 3, 1))));
    }

    #[test]
    fn dates_with_another_offset_compare_by_time() {
        let f = RetrievalFilter::default().with_modified_range(Some(at(2025, 3, 1)), None);
        assert!(f.matches("/a", "t", Some("2025-03-01T01:00:00+01:00")));
        assert!(!f.matches("/a", "t", Some("2025-03-01T00:59:59+01:00")));
    }

    #[test]
    fn inverted_range_matches_nothing() {
        let f = RetrievalFilter::default().with_modified_range(Some(at(2025, 4, 1)), Some(at(2025, 3, 1)));
        assert!(!f.matches("/a", "t", Some(&stamp(2025, 3, 15))));
        assert!(!f.matches("/a", "t", Some(&stamp(2025, 5, 1))));
    }

    #[test]
    fn missing_or_bad_date_never_matches_a_date_filter() {
        let f = RetrievalFilter::default().with_modified_range(Some(at(2025, 3, 1)), None);
        assert!(!f.matches("/a", "t", None));
        assert!(!f.matches("/a", "t", Some("not a date")));
    }

    #[test]
    fn unparsable_bound_matches_nothing() {
        let f = RetrievalFilter { modified_from: Some("yesterday".to_string()), ..Default::default() };
        assert!(!f.matches("/a", "t", Some(&stamp(2025, 3, 1))));
        assert!(f.lance_predicate().unwrap().contains("false"));
    }

    #[test]
    fn content_types_and_dimensions_combine() {
        let f = RetrievalFilter {
            sources: vec!["/a".to_string()],
            content_types: vec!["Code".to_string(), "Markdown".to_string()],
            ..Default::default()
        }
        .with_modified_range(Some(at(2025, 1, 1)), None);
        assert!(f.matches("/a/x.rs", "Code", Some(&stamp(2025, 6, 1))));
        assert!(!f.matches("/a/x.pdf", "PDF", Some(&stamp(2025, 6, 1))));
        assert!(!f.matches("/b/x.rs", "Code", Some(&stamp(2025, 6, 1))));
        assert!(!f.matches("/a/x.rs", "Code", Some(&stamp(2024, 6, 1))));
        let pred = f.lance_predicate().unwrap();
        assert!(pred.contains(" AND content_type IN ('Code', 'Markdown')"), "{pred}");
    }
}
