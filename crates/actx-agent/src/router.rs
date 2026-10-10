//! # 🧭 Chat ModelRouter & Dynamic Query Complexity Classifier
//!
//! Sub-millisecond (<1µs) deterministic classifier and source-aware intent router.
//! Dynamically evaluates user prompts to select the optimal execution mode (`Fast RAG` vs `Deep Search`)
//! in `SearchMode::Auto`, while detecting query intent (Code, Architecture, Document, General).

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use crate::config::{AgentExecutionMode, SearchMode};

/// Assessed complexity tier for a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryComplexity {
    /// Single-turn, low-latency factual retrieval or syntax lookup.
    Fast,
    /// Multi-turn, multi-phase reflective reasoning (RFC-042).
    Deep,
}

/// Semantic intent domain detected from the user prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntent {
    /// Code implementation, debugging, refactoring, or programming syntax.
    Code,
    /// Cross-module system flows, architectural diagrams, or component lifecycle.
    Architecture,
    /// Business documents, legal contracts, medical reports, or fiscal records.
    Document,
    /// General knowledge, conversational greetings, or conceptual questions.
    General,
}

impl QueryIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            QueryIntent::Code => "code",
            QueryIntent::Architecture => "architecture",
            QueryIntent::Document => "document",
            QueryIntent::General => "general",
        }
    }
}

/// Final routing decision emitted per user query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Recommended agent execution mode (Direct, ReAct, or DeepSearch).
    pub mode: AgentExecutionMode,
    /// Complexity category (Fast vs Deep).
    pub complexity: QueryComplexity,
    /// Detected intent domain.
    pub intent: QueryIntent,
    /// Confidence score in [0.0, 1.0].
    pub confidence: f32,
    /// Human-readable rationale for telemetry and UI badges.
    pub reason: String,
}

/// Abstract contract for query complexity classification.
pub trait ComplexityClassifier: Send + Sync {
    /// Classifies complexity and intent of a raw query string.
    fn classify(&self, query: &str) -> (QueryComplexity, QueryIntent, f32, String);
}

/// Configuration parameters for the Chat ModelRouter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRouterConfig {
    /// Threshold score above which `Auto` mode routes to `DeepSearch` (default: 0.45).
    pub deep_threshold: f32,
    /// Word count boundary below which short queries favor `Fast` (default: 10).
    pub fast_max_words: usize,
    /// Word count boundary above which long multi-clause queries favor `Deep` (default: 25).
    pub deep_min_words: usize,
}

impl Default for ChatRouterConfig {
    fn default() -> Self {
        Self {
            deep_threshold: 0.45,
            fast_max_words: 10,
            deep_min_words: 25,
        }
    }
}

/// High-speed deterministic classifier based on structural and multilingual lexical signals.
#[derive(Debug, Clone)]
pub struct DeterministicClassifier {
    pub config: ChatRouterConfig,
}

impl Default for DeterministicClassifier {
    fn default() -> Self {
        Self {
            config: ChatRouterConfig::default(),
        }
    }
}

impl DeterministicClassifier {
    pub fn new(config: ChatRouterConfig) -> Self {
        Self { config }
    }

    /// Evaluates structural signals (query length, multi-question markers, comparative syntax).
    fn evaluate_structural_score(&self, query: &str) -> (f32, Vec<&'static str>) {
        let mut score: f32 = 0.0;
        let mut reasons = Vec::new();

        let words: Vec<&str> = query.split_whitespace().collect();
        let word_count = words.len();

        if word_count >= self.config.deep_min_words {
            score += 0.25;
            reasons.push("detailed query length");
        } else if word_count <= self.config.fast_max_words {
            score -= 0.15;
        }

        // Multi-question or multi-aspect markers ("? ... ?", "and also", "as well as", etc.)
        let q_marks = query.chars().filter(|&c| c == '?').count();
        if q_marks >= 2 {
            score += 0.25;
            reasons.push("multiple questions in single prompt");
        }

        let lower = query.to_lowercase();
        if lower.contains(" e também ") || lower.contains(" além de ") || lower.contains(" bem como ")
            || lower.contains(" and also ") || lower.contains(" as well as ")
            || lower.contains(" por que e como ") || lower.contains(" why and how ")
        {
            score += 0.20;
            reasons.push("multi-aspect additive conjunctions");
        }

        // Comparative markers ("difference between", "compare", "versus", "vs", "pros and cons", "trade-offs")
        if lower.contains("diferença entre") || lower.contains("difference between")
            || lower.contains("compare") || lower.contains("comparar")
            || lower.contains(" versus ") || lower.contains(" vs ")
            || lower.contains("prós e contras") || lower.contains("pros and cons")
            || lower.contains("trade-off") || lower.contains("tradeoff")
            || lower.contains("qual a melhor abordagem") || lower.contains("which is better")
        {
            score += 0.35;
            reasons.push("comparative analysis / trade-offs");
        }

        (score, reasons)
    }

    /// Evaluates domain lexical triggers.
    fn evaluate_lexical_score(&self, query: &str) -> (f32, Vec<&'static str>, QueryIntent) {
        let mut score: f32 = 0.0;
        let mut reasons = Vec::new();
        let lower = query.to_lowercase();

        // Deep Lexical Triggers (PT & EN)
        let deep_triggers = [
            ("arquitetura", "system architecture"),
            ("architecture", "system architecture"),
            ("end-to-end", "end-to-end flow"),
            ("ponta a ponta", "end-to-end flow"),
            ("fluxo completo", "complete flow"),
            ("ciclo de vida", "lifecycle"),
            ("lifecycle", "system lifecycle"),
            ("auditoria", "audit"),
            ("segurança", "security analysis"),
            ("refatoração", "refactoring strategy"),
            ("refactoring", "refactoring analysis"),
            ("todas as ocorrências", "exhaustive occurrences lookup"),
            ("todas as referências", "reference lookup"),
            ("quais módulos", "multi-module analysis"),
            ("quais arquivos", "multi-file analysis"),
            ("como se relacionam", "structural correlation"),
            ("impacto de mudar", "impact analysis"),
            ("passo a passo detalhado", "step-by-step reasoning"),
            ("diagnóstico completo", "comprehensive diagnostics"),
        ];

        for (trigger, label) in deep_triggers {
            if lower.contains(trigger) {
                score += 0.30;
                reasons.push(label);
            }
        }

        // Fast Lexical Triggers (PT & EN)
        let fast_triggers = [
            "onde está", "onde fica", "where is", "where's",
            "qual a porta", "what port", "qual linha", "which line",
            "qual comando", "qual atalho", "como importar", "how to import",
            "o que significa", "what does", "qual a versão", "what version",
            "valor padrão", "default value", "definido em", "defined in",
            "quem é", "who is", "mostre a função", "show function",
        ];

        for trigger in fast_triggers {
            if lower.contains(trigger) {
                score -= 0.30;
                reasons.push("targeted fast lookup query");
            }
        }

        // Intent detection
        let intent = if lower.contains("arquitetura") || lower.contains("architecture")
            || lower.contains("fluxo") || lower.contains("pipeline")
            || lower.contains("diagrama") || lower.contains("hexagonal")
            || lower.contains("fsm") || lower.contains("orquestrador")
        {
            QueryIntent::Architecture
        } else if lower.contains("fn ") || lower.contains("def ") || lower.contains("class ")
            || lower.contains("struct ") || lower.contains("impl ") || lower.contains("interface ")
            || lower.contains("::") || lower.contains("pub ") || lower.contains("async ")
            || lower.contains("bug") || lower.contains("erro ") || lower.contains("error ")
            || lower.contains("panic") || lower.contains("exception") || lower.contains("test")
            || lower.contains("código") || lower.contains("code") || lower.contains("função")
        {
            QueryIntent::Code
        } else if lower.contains("contrato") || lower.contains("fatura") || lower.contains("invoice")
            || lower.contains("cmr") || lower.contains("paciente") || lower.contains("prescrição")
            || lower.contains("balancete") || lower.contains("relatório") || lower.contains("artigo")
            || lower.contains("lei") || lower.contains("processo") || lower.contains("documento")
        {
            QueryIntent::Document
        } else {
            QueryIntent::General
        };

        (score, reasons, intent)
    }
}

impl ComplexityClassifier for DeterministicClassifier {
    fn classify(&self, query: &str) -> (QueryComplexity, QueryIntent, f32, String) {
        let (struct_score, struct_reasons) = self.evaluate_structural_score(query);
        let (lex_score, lex_reasons, intent) = self.evaluate_lexical_score(query);

        // Normalize base score around 0.20 (baseline prior for questions)
        let total_score = (0.20 + struct_score + lex_score).clamp(0.0, 1.0);

        let mut all_reasons = struct_reasons;
        all_reasons.extend(lex_reasons);

        let complexity = if total_score >= self.config.deep_threshold {
            QueryComplexity::Deep
        } else {
            QueryComplexity::Fast
        };

        let reason_str = if all_reasons.is_empty() {
            if complexity == QueryComplexity::Deep {
                "Complex multi-dependency query pattern".to_string()
            } else {
                "Direct low-latency point query".to_string()
            }
        } else {
            all_reasons.join(", ")
        };

        (complexity, intent, total_score, reason_str)
    }
}

/// Central Chat ModelRouter managing query evaluation, mode resolution, and intent routing.
#[derive(Clone)]
pub struct ChatModelRouter {
    classifier: Arc<dyn ComplexityClassifier>,
    config: ChatRouterConfig,
}

impl Default for ChatModelRouter {
    fn default() -> Self {
        let config = ChatRouterConfig::default();
        Self {
            classifier: Arc::new(DeterministicClassifier::new(config.clone())),
            config,
        }
    }
}

impl ChatModelRouter {
    pub fn new(config: ChatRouterConfig) -> Self {
        Self {
            classifier: Arc::new(DeterministicClassifier::new(config.clone())),
            config,
        }
    }

    pub fn with_classifier(mut self, classifier: Arc<dyn ComplexityClassifier>) -> Self {
        self.classifier = classifier;
        self
    }

    pub fn config(&self) -> &ChatRouterConfig {
        &self.config
    }

    /// Evaluates the user query against the active SearchMode policy.
    pub fn evaluate(&self, query: &str, search_mode: SearchMode) -> RoutingDecision {
        let (complexity, intent, confidence, reason) = self.classifier.classify(query);

        let (mode, final_complexity, final_reason) = match search_mode {
            SearchMode::Fast => (
                AgentExecutionMode::ReAct,
                QueryComplexity::Fast,
                format!("⚡ Fast mode forced by user (/search fast): {}", reason),
            ),
            SearchMode::Deep => (
                AgentExecutionMode::DeepSearch,
                QueryComplexity::Deep,
                format!("🧠 Deep Search mode forced by user (/search deep): {}", reason),
            ),
            SearchMode::Auto => match complexity {
                QueryComplexity::Deep => (
                    AgentExecutionMode::DeepSearch,
                    QueryComplexity::Deep,
                    format!("🧠 Autonomously routed to Deep Search (score: {:.2}): {}", confidence, reason),
                ),
                QueryComplexity::Fast => (
                    AgentExecutionMode::ReAct,
                    QueryComplexity::Fast,
                    format!("⚡ Autonomously routed to Fast RAG (score: {:.2}): {}", confidence, reason),
                ),
            },
        };

        RoutingDecision {
            mode,
            complexity: final_complexity,
            intent,
            confidence,
            reason: final_reason,
        }
    }
}
