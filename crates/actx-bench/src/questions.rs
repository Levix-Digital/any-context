//! The hand-labeled question file and its validation against the corpus.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::corpus::{normalize_whitespace, Corpus};

pub const LANGUAGES: &[&str] = &["pt", "en"];
pub const CATEGORIES: &[&str] = &["factual", "version", "form-field", "cross-language", "negative"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relevant {
    /// Path relative to the corpus root, with `/` separators.
    pub doc: String,
    /// 1 related, 2 partial answer, 3 full answer.
    pub grade: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Question {
    pub id: String,
    pub lang: String,
    pub category: String,
    pub text: String,
    #[serde(default)]
    pub relevant: Vec<Relevant>,
}

impl Question {
    pub fn is_negative(&self) -> bool {
        self.category == "negative"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuestionFile {
    pub questions: Vec<Question>,
}

impl QuestionFile {
    pub fn parse(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("questions file is not valid: {e}"))
    }
}

/// One broken label, named by the question it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelError {
    pub question: String,
    pub reason: String,
}

impl std::fmt::Display for LabelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.question, self.reason)
    }
}

/// Checks every question against the corpus. Passages are checked here for documents whose text
/// the corpus can read directly; the others (PDF) are checked after indexing, against their chunks.
pub fn validate(file: &QuestionFile, corpus: &Corpus) -> Vec<LabelError> {
    let mut errors = Vec::new();
    let mut seen = HashSet::new();
    for q in &file.questions {
        let mut fail = |reason: String| errors.push(LabelError { question: q.id.clone(), reason });
        if q.id.trim().is_empty() {
            fail("empty id".into());
        } else if !seen.insert(q.id.clone()) {
            fail("duplicate id".into());
        }
        if !LANGUAGES.contains(&q.lang.as_str()) {
            fail(format!("unknown language '{}'", q.lang));
        }
        if !CATEGORIES.contains(&q.category.as_str()) {
            fail(format!("unknown category '{}'", q.category));
        }
        if q.text.trim().is_empty() {
            fail("empty question text".into());
        }
        if q.is_negative() && !q.relevant.is_empty() {
            fail("a negative question has relevant documents".into());
        }
        if !q.is_negative() && q.relevant.is_empty() {
            fail("a positive question has no relevant document".into());
        }
        for r in &q.relevant {
            if !(1..=3).contains(&r.grade) {
                fail(format!("grade {} of '{}' is not 1, 2 or 3", r.grade, r.doc));
            }
            if !corpus.contains(&r.doc) {
                fail(format!("document '{}' is not in the corpus", r.doc));
                continue;
            }
            if let (Some(passage), Some(text)) = (&r.passage, corpus.plain_text(&r.doc)) {
                if !normalize_whitespace(&text).contains(&normalize_whitespace(passage)) {
                    fail(format!("passage not found in '{}': \"{passage}\"", r.doc));
                }
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::tests::temp_corpus;

    fn q(id: &str, category: &str, relevant: Vec<Relevant>) -> Question {
        Question { id: id.into(), lang: "pt".into(), category: category.into(), text: "pergunta".into(), relevant }
    }

    fn rel(doc: &str, grade: u8, passage: Option<&str>) -> Relevant {
        Relevant { doc: doc.into(), grade, passage: passage.map(String::from) }
    }

    fn reasons(file: QuestionFile) -> Vec<String> {
        let (_dir, corpus) = temp_corpus(&[("pt/a.md", "O prazo  de entrega\né de 5 dias."), ("pt/b.pdf", "%PDF")]);
        validate(&file, &corpus).into_iter().map(|e| e.to_string()).collect()
    }

    #[test]
    fn valid_file_has_no_errors() {
        let file = QuestionFile {
            questions: vec![
                q("pt-a-001", "factual", vec![rel("pt/a.md", 3, Some("prazo de entrega é de 5"))]),
                q("pt-a-002", "factual", vec![rel("pt/b.pdf", 2, Some("anything: checked after indexing"))]),
                q("pt-neg-001", "negative", vec![]),
            ],
        };
        assert!(reasons(file).is_empty());
    }

    #[test]
    fn reports_each_broken_label() {
        let mut bad_lang = q("x-004", "factual", vec![rel("pt/a.md", 1, None)]);
        bad_lang.lang = "fr".into();
        let file = QuestionFile {
            questions: vec![
                q("x-001", "factual", vec![rel("pt/missing.md", 3, None)]),
                q("x-002", "factual", vec![rel("pt/a.md", 3, Some("prazo de 10 dias"))]),
                q("x-003", "negative", vec![rel("pt/a.md", 3, None)]),
                bad_lang,
                q("x-005", "factual", vec![]),
                q("x-005", "factual", vec![rel("pt/a.md", 4, None)]),
                q("x-006", "opinion", vec![rel("pt/a.md", 1, None)]),
            ],
        };
        assert_eq!(
            reasons(file),
            vec![
                "x-001: document 'pt/missing.md' is not in the corpus",
                "x-002: passage not found in 'pt/a.md': \"prazo de 10 dias\"",
                "x-003: a negative question has relevant documents",
                "x-004: unknown language 'fr'",
                "x-005: a positive question has no relevant document",
                "x-005: duplicate id",
                "x-005: grade 4 of 'pt/a.md' is not 1, 2 or 3",
                "x-006: unknown category 'opinion'",
            ]
        );
    }

    #[test]
    fn parses_the_documented_format() {
        let file = QuestionFile::parse(
            r#"{"questions":[{"id":"en-neg-001","lang":"en","category":"negative","text":"t"},
               {"id":"en-a-001","lang":"en","category":"factual","text":"t","relevant":[{"doc":"en/a.md","grade":3,"passage":"p"}]}]}"#,
        )
        .unwrap();
        assert!(file.questions[0].relevant.is_empty());
        assert_eq!(file.questions[1].relevant[0], rel("en/a.md", 3, Some("p")));
        assert!(QuestionFile::parse("{").is_err());
    }
}
