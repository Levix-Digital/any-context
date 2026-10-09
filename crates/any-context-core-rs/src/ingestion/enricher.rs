use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use sha2::{Digest, Sha256};
use crate::models::SemanticEnvelope;
use crate::storage::sqlite::NativeConfigDb;

/// Universal Native Contextual Enrichment Engine.
/// Provides high-speed extractive document summarization, domain keyword extraction,
/// and Contextual Retrieval chunk enveloping with persistent SQLite caching.
#[derive(Clone)]
pub struct NativeContextualEnricher {
    db: Arc<NativeConfigDb>,
}

impl NativeContextualEnricher {
    pub fn new(db: Arc<NativeConfigDb>) -> Self {
        Self { db }
    }

    /// Computes SHA-256 hash of text content.
    pub fn compute_hash(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.trim().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Extracts or retrieves cached SemanticEnvelope for document text.
    pub fn extract_envelope(
        &self,
        doc_text: &str,
        file_name: &str,
        file_path: Option<&str>,
        url: Option<&str>,
    ) -> SemanticEnvelope {
        let clean = doc_text.trim();
        if clean.is_empty() {
            return SemanticEnvelope {
                summary: format!("Empty document '{}'", file_name),
                keywords: Vec::new(),
                content_hash: "empty".to_string(),
                file_name: file_name.to_string(),
                file_path: file_path.map(|s| s.to_string()),
                url: url.map(|s| s.to_string()),
                created_at: chrono::Utc::now().to_rfc3339(),
            };
        }

        let hash = Self::compute_hash(clean);
        if let Ok(Some(cached)) = self.db.get_semantic_envelope(&hash) {
            return cached;
        }

        let lines: Vec<&str> = clean.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

        // 1. Extract Title / Lead Heading candidates
        let mut title_candidate = None;
        for line in lines.iter().take(10) {
            if line.starts_with('#') {
                let cleaned = line.trim_start_matches('#').trim();
                if cleaned.len() > 2 {
                    title_candidate = Some(cleaned.to_string());
                    break;
                }
            } else if line.len() > 3 && line.len() < 100 && !line.ends_with('.') {
                title_candidate = Some(line.to_string());
                break;
            }
        }
        let doc_title = title_candidate.unwrap_or_else(|| file_name.to_string());

        // 2. Extract Lead Descriptive Sentences
        let lead_text = lines.iter().take(12).copied().collect::<Vec<&str>>().join(" ");
        let sentences: Vec<&str> = lead_text
            .split(&['.', '!', '?'][..])
            .map(|s| s.trim())
            .filter(|s| s.len() > 15)
            .collect();

        let lead_summary = if !sentences.is_empty() {
            sentences.iter().take(3).copied().collect::<Vec<&str>>().join(". ") + "."
        } else {
            let truncated: String = clean.chars().take(350).collect();
            truncated
        };

        let mut summary = format!("Document '{}': {}", doc_title, lead_summary);
        if summary.len() > 450 {
            let mut cut: String = summary.chars().take(447).collect();
            cut.push_str("...");
            summary = cut;
        }

        // 3. Extract Top Domain Keywords
        let keywords = Self::extract_top_keywords(clean, &doc_title, 7);

        let envelope = SemanticEnvelope {
            summary,
            keywords,
            content_hash: hash,
            file_name: file_name.to_string(),
            file_path: file_path.map(|s| s.to_string()),
            url: url.map(|s| s.to_string()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let _ = self.db.save_semantic_envelope(&envelope);
        envelope
    }

    /// Envelopes chunk text with the authoritative document summary and keywords header.
    pub fn apply_envelope_to_chunk(&self, chunk_text: &str, envelope: &SemanticEnvelope) -> String {
        let kw_str = if envelope.keywords.is_empty() {
            "general".to_string()
        } else {
            envelope.keywords.join(", ")
        };
        format!("[Context: {} | Keywords: {}]\n---\n{}", envelope.summary, kw_str, chunk_text)
    }

    /// Extracts top domain keywords by frequency, length, and title significance.
    pub fn extract_top_keywords(text: &str, title: &str, top_n: usize) -> Vec<String> {
        let stop_words: HashSet<&'static str> = [
            "the", "and", "for", "with", "from", "that", "this", "then", "into", "over",
            "between", "after", "before", "about", "have", "been", "were", "what", "which",
            "when", "where", "there", "their", "these", "those", "should", "could", "would",
            "para", "com", "como", "esta", "este", "esse", "isso", "pelo", "pela", "onde",
            "quando", "mais", "sobre", "entre", "depois", "ainda", "muito", "mesmo", "todos",
            "self", "true", "false", "null", "none", "void", "impl", "fn", "let", "mut",
        ].into_iter().collect();

        let word_re = regex::Regex::new(r"(?u)\b[\p{L}]{4,}\b").unwrap();
        let mut freq: HashMap<String, usize> = HashMap::new();

        for mat in word_re.find_iter(text) {
            let lower = mat.as_str().to_lowercase();
            if !stop_words.contains(lower.as_str()) {
                *freq.entry(lower).or_insert(0) += 1;
            }
        }

        // Boost title words
        for mat in word_re.find_iter(title) {
            let lower = mat.as_str().to_lowercase();
            if !stop_words.contains(lower.as_str()) {
                *freq.entry(lower).or_insert(0) += 5;
            }
        }

        let mut sorted: Vec<(String, usize)> = freq.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        sorted.into_iter().take(top_n).map(|(k, _)| k).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_top_keywords() {
        let text = "Distributed Consensus Engine handles leader election and raft log replication. The leader coordinates consensus.";
        let keywords = NativeContextualEnricher::extract_top_keywords(text, "Consensus Engine", 4);
        assert!(!keywords.is_empty());
        assert!(keywords.contains(&"consensus".to_string()) || keywords.contains(&"leader".to_string()));
    }

    #[test]
    fn test_enricher_cache_and_envelope() {
        let db = Arc::new(NativeConfigDb::open_in_memory().unwrap());
        let enricher = NativeContextualEnricher::new(db);
        let doc = "# Architecture Blueprint\nThis system provides high-performance vector search and distributed storage.\nIt is designed for zero latency.";
        let env = enricher.extract_envelope(doc, "arch.md", Some("docs/arch.md"), None);
        assert!(env.summary.contains("Architecture Blueprint"));
        assert!(!env.keywords.is_empty());

        let enveloped = enricher.apply_envelope_to_chunk("search_latency = 12ms", &env);
        assert!(enveloped.starts_with("[Context:"));
        assert!(enveloped.contains("search_latency = 12ms"));
    }
}
