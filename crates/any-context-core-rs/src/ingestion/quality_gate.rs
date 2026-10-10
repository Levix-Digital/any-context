//! High-Performance Deterministic Quality Gate for Ingestion Pipeline.
//!
//! Evaluates extracted chunks in sub-0.1ms using Shannon entropy, lexical density,
//! syntactic structure analysis, and layout density signals.
//! Discards noise, boilerplate, minified blobs, and degenerate fragments, while
//! dispatching dense complex forms and visual diagrams to Document AI.

use std::collections::HashSet;
use crate::models::ChunkPayload;

/// Represents the final decision emitted by the Quality Gate for a given chunk.
#[derive(Debug, Clone, PartialEq)]
pub enum QualityDecision {
    /// Chunk has sufficient semantic density and passes into vectorization.
    Pass { score: f32 },
    /// Chunk is noise or degenerate and must be discarded from index.
    Reject { reason: QualityRejectionReason, score: f32 },
    /// Chunk requires Document AI / Specialized layout handling.
    NeedsDocumentAi { target: DocumentAiTarget },
}

/// Specific, auditable reason for rejecting a chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QualityRejectionReason {
    EmptyOrTooShort,
    LowEntropyRepeatedPattern,
    HighEntropyBinaryNoise,
    MinifiedOrCompressedLine,
    DegenerateSyntaxFragment,
    BoilerplateSpam,
}

/// Target category for Document AI processing.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentAiTarget {
    /// Scanned PDF page or rasterized canvas without textual stream.
    ScannedPdfPage { page_num: usize },
    /// Complex multi-column form or table with detached labels and values.
    DenseComplexForm { template_ratio: f32 },
    /// Technical diagram, schematic, chart, or architecture visual.
    VisualDiagram { width: u32, height: u32 },
}

/// Configuration thresholds for the Quality Gate.
#[derive(Debug, Clone)]
pub struct QualityGateConfig {
    pub min_chars: usize,
    pub min_entropy: f64,
    pub max_entropy: f64,
    pub min_lexical_density: f64,
    pub max_single_line_chars: usize,
    pub max_table_pipe_ratio: f32,
}

impl Default for QualityGateConfig {
    fn default() -> Self {
        Self {
            min_chars: 20,
            min_entropy: 1.5,
            max_entropy: 5.95,
            min_lexical_density: 0.12,
            max_single_line_chars: 2000,
            max_table_pipe_ratio: 0.55,
        }
    }
}

/// Deterministic Quality Gate Engine.
#[derive(Debug, Clone)]
pub struct QualityGate {
    config: QualityGateConfig,
}

impl Default for QualityGate {
    fn default() -> Self {
        Self::new(QualityGateConfig::default())
    }
}

impl QualityGate {
    pub fn new(config: QualityGateConfig) -> Self {
        Self { config }
    }

    /// Evaluates a single chunk and outputs a `QualityDecision`.
    pub fn evaluate_chunk(&self, chunk: &ChunkPayload) -> QualityDecision {
        let raw_text = &chunk.text;

        // Extract body text, stripping the leading `// Context: ...` breadcrumb if present
        let body_text = Self::strip_breadcrumb(raw_text);
        let trimmed_body = body_text.trim();

        // 1. Check for Document AI targets first based on content_type and metadata
        if chunk.content_type == "pdf_scan" {
            return QualityDecision::NeedsDocumentAi {
                target: DocumentAiTarget::ScannedPdfPage {
                    page_num: chunk.start_line.max(1),
                },
            };
        }

        if chunk.content_type == "visual_diagram" {
            let (w, h) = Self::parse_diagram_resolution(raw_text);
            if w >= 150 && h >= 150 {
                return QualityDecision::NeedsDocumentAi {
                    target: DocumentAiTarget::VisualDiagram { width: w, height: h },
                };
            }
        }

        // 2. Length check: Too short or empty
        if trimmed_body.len() < self.config.min_chars {
            return QualityDecision::Reject {
                reason: QualityRejectionReason::EmptyOrTooShort,
                score: 0.0,
            };
        }

        // 3. Degenerate Syntax Fragment check (e.g. solitary closing brackets "}", "};", ")]")
        if self.is_degenerate_syntax(trimmed_body) {
            return QualityDecision::Reject {
                reason: QualityRejectionReason::DegenerateSyntaxFragment,
                score: 0.05,
            };
        }

        // 4. Minified or compressed line check (> max_single_line_chars with low line breaks)
        if self.has_minified_monolithic_line(trimmed_body) {
            return QualityDecision::Reject {
                reason: QualityRejectionReason::MinifiedOrCompressedLine,
                score: 0.10,
            };
        }

        // 5. Shannon Entropy evaluation
        let entropy = self.compute_shannon_entropy(trimmed_body);
        if entropy < self.config.min_entropy {
            return QualityDecision::Reject {
                reason: QualityRejectionReason::LowEntropyRepeatedPattern,
                score: (entropy / 8.0) as f32,
            };
        }
        if (entropy > self.config.max_entropy && self.has_high_non_ascii_density(trimmed_body))
            || self.has_binary_control_noise(trimmed_body)
        {
            return QualityDecision::Reject {
                reason: QualityRejectionReason::HighEntropyBinaryNoise,
                score: 0.15,
            };
        }

        // 6. Lexical Density check (unique words / total words)
        let lexical_density = self.compute_lexical_density(trimmed_body);
        if lexical_density < self.config.min_lexical_density && trimmed_body.len() > 150 {
            return QualityDecision::Reject {
                reason: QualityRejectionReason::LowEntropyRepeatedPattern,
                score: lexical_density as f32,
            };
        }

        // 7. Check for Dense Complex Forms (like IKEA CMRs, invoices, tabular forms with disjoint pipes)
        if (chunk.content_type == "pdf" || chunk.content_type == "document_form")
            && self.is_dense_complex_form(trimmed_body)
        {
            let pipe_ratio = self.compute_pipe_ratio(trimmed_body);
            return QualityDecision::NeedsDocumentAi {
                target: DocumentAiTarget::DenseComplexForm {
                    template_ratio: pipe_ratio,
                },
            };
        }

        // Calculate normalized quality score
        let score = (0.70 + (lexical_density * 0.20) + (entropy.min(4.5) / 4.5 * 0.10)).min(1.0) as f32;
        QualityDecision::Pass { score }
    }

    /// Computes Shannon entropy in bits per character based on UTF-8 byte frequency distribution.
    pub fn compute_shannon_entropy(&self, text: &str) -> f64 {
        if text.is_empty() {
            return 0.0;
        }

        let mut byte_counts = [0usize; 256];
        let mut total_bytes = 0usize;

        for &b in text.as_bytes() {
            byte_counts[b as usize] += 1;
            total_bytes += 1;
        }

        if total_bytes == 0 {
            return 0.0;
        }

        let total_f = total_bytes as f64;
        let mut entropy = 0.0;

        for &count in &byte_counts {
            if count > 0 {
                let p = (count as f64) / total_f;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Computes lexical density as the ratio of unique word tokens to total word tokens.
    pub fn compute_lexical_density(&self, text: &str) -> f64 {
        let words: Vec<&str> = text
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();

        if words.is_empty() {
            return 0.0;
        }

        let total = words.len();
        let unique_words: HashSet<&str> = words.into_iter().collect();

        (unique_words.len() as f64) / (total as f64)
    }

    /// Checks if the content is composed of minified code lines exceeding the max length.
    pub fn has_minified_monolithic_line(&self, text: &str) -> bool {
        for line in text.lines() {
            if line.len() > self.config.max_single_line_chars {
                // Check if it's not simply a wide markdown table row with many fields
                let pipe_count = line.chars().filter(|&c| c == '|').count();
                if pipe_count < 10 {
                    return true;
                }
            }
        }
        false
    }

    /// Checks whether the text consists only of degenerate syntax like lonely braces.
    pub fn is_degenerate_syntax(&self, text: &str) -> bool {
        let filtered: String = text
            .chars()
            .filter(|c| !c.is_whitespace() && *c != ';')
            .collect();

        if filtered.is_empty() {
            return false;
        }

        if filtered.len() <= 4 && filtered.chars().all(|c| matches!(c, '}' | ']' | ')' | '{' | '[' | '(')) {
            return true;
        }

        false
    }

    /// Detects if text contains null bytes or anomalous control character density.
    pub fn has_binary_control_noise(&self, text: &str) -> bool {
        let control_count = text
            .chars()
            .filter(|&c| c == '\0' || (c < ' ' && c != '\t' && c != '\n' && c != '\r'))
            .count();
        control_count > 0 && (control_count as f64 / text.chars().count().max(1) as f64) > 0.05
    }

    /// Detects if non-ASCII byte density is unusually high, signaling corrupted binary garbage.
    pub fn has_high_non_ascii_density(&self, text: &str) -> bool {
        let non_ascii_count = text.chars().filter(|c| !c.is_ascii()).count();
        let total_chars = text.chars().count().max(1);
        (non_ascii_count as f64 / total_chars as f64) > 0.45
    }

    /// Evaluates if a text represents a dense tabular form with fragmented cell pipes.
    pub fn is_dense_complex_form(&self, text: &str) -> bool {
        let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return false;
        }

        let pipe_lines = lines.iter().filter(|l| l.contains('|')).count();
        let pipe_line_ratio = (pipe_lines as f32) / (lines.len() as f32);

        // Also check table delimiter ratio (`| --- | --- |`)
        let table_header_separators = lines.iter().filter(|l| l.contains("| ---") || l.contains("|:---")).count();

        pipe_line_ratio >= self.config.max_table_pipe_ratio || table_header_separators >= 3
    }

    /// Computes ratio of pipe occurrences.
    pub fn compute_pipe_ratio(&self, text: &str) -> f32 {
        let lines: Vec<&str> = text.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
        if lines.is_empty() {
            return 0.0;
        }
        let pipe_lines = lines.iter().filter(|l| l.contains('|')).count();
        (pipe_lines as f32) / (lines.len() as f32)
    }

    /// Strips leading `// Context: ...` breadcrumb.
    pub fn strip_breadcrumb(text: &str) -> &str {
        if !text.starts_with("// Context:") {
            return text;
        }

        if let Some(pos) = text.find("\n---\n") {
            return &text[pos + 5..];
        }

        if let Some(pos) = text.find("\n\n") {
            return &text[pos + 2..];
        }

        if let Some(pos) = text.find('\n') {
            return &text[pos + 1..];
        }

        text
    }

    /// Extracts resolution from visual diagram headers like `[Resolution: 1920x1080 px]`.
    fn parse_diagram_resolution(text: &str) -> (u32, u32) {
        if let Some(start) = text.find("Resolution: ") {
            let rest = &text[start + 12..];
            if let Some(end) = rest.find(" px") {
                let res_str = &rest[..end];
                let parts: Vec<&str> = res_str.split('x').map(|s| s.trim()).collect();
                if parts.len() == 2 {
                    if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                        return (w, h);
                    }
                }
            }
        }
        (0, 0)
    }
}
