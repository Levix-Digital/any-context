//! Query Complexity & Intent Classifier (deterministic heuristic, ADR-130).
//!
//! Routes queries with a sub-microsecond keyword heuristic. Downloadable BGE-Small
//! weights are tracked but not executed until neural routing is implemented.
//! Performs semantic intent classification (Code, Architecture, Document, General)
//! and dynamically routes between Fast RAG and Deep Search in sub-15ms.

use std::path::Path;
use crate::session::ModelSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnnxComplexity {
    Fast,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnnxIntent {
    Code,
    Architecture,
    DevOpsInfra,
    DataSchema,
    SecurityAudit,
    Document,
    General,
}

impl OnnxIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Architecture => "architecture",
            Self::DevOpsInfra => "devops_infra",
            Self::DataSchema => "data_schema",
            Self::SecurityAudit => "security_audit",
            Self::Document => "document",
            Self::General => "general",
        }
    }
}

/// Query complexity & intent classifier (deterministic heuristic).
///
/// The BGE-Small weights can be downloaded, but neural classification is not implemented
/// yet (ADR-130): every query goes through the same sub-microsecond keyword heuristic.
#[derive(Debug, Clone)]
pub struct OnnxQueryClassifier {
    weights: Option<ModelSession>,
}

impl OnnxQueryClassifier {
    /// Initializes the classifier, recording whether `bge_small_query_int8.onnx` is present.
    pub fn new(model_path: &Path) -> Self {
        let weights = ModelSession::open(model_path, "bge-small-onnx").ok();
        Self { weights }
    }

    /// Creates an instance directly with an existing weight descriptor.
    pub fn from_session(weights: Option<ModelSession>) -> Self {
        Self { weights }
    }

    /// Always `false`: classification is heuristic in this build (ADR-130).
    pub fn is_neural_active(&self) -> bool {
        false
    }

    /// True when the BGE-Small weights are present on disk (not executed yet).
    pub fn weights_present(&self) -> bool {
        self.weights.as_ref().map(|s| s.is_valid).unwrap_or(false)
    }

    /// Evaluates the query string and produces a classification decision.
    pub fn classify_query(&self, query: &str) -> (OnnxComplexity, OnnxIntent, f32, String) {
        let q_trimmed = query.trim();
        let q_lower = q_trimmed.to_lowercase();
        let word_count = q_trimmed.split_whitespace().count();

        let (intent, _intent_weight) = Self::detect_intent(&q_lower);
        let (complexity, confidence, reason) = Self::evaluate_fallback_complexity(&q_lower, word_count, intent);

        (complexity, intent, confidence, reason)
    }

    fn detect_intent(q: &str) -> (OnnxIntent, f32) {
        let sec_keywords = [
            "security", "segurança", "vulnerability", "vulnerabilidade", "cve", "secret", "segredo",
            "api_key", "password", "senha", "auth", "autenticação", "jwt", "sanitiz", "injection",
            "injeção", "lgpd", "gdpr", "permission", "permissão"
        ];
        let devops_keywords = [
            "docker", "dockerfile", "github action", "workflow", "ci/cd", "ci ", "cd ", "deploy",
            "sccache", "environment variable", "variável de ambiente", "env var", "port ", "porta ",
            "bash", "powershell", "terminal", "kubernetes", "k8s", "container"
        ];
        let data_keywords = [
            "sql", "sqlite", "lancedb", "postgres", "mysql", "table", "tabela", "column", "coluna",
            "schema", "esquema", "migration", "migração", "primary key", "foreign key", "relational",
            "tabular", "database", "banco de dados"
        ];
        let arch_keywords = [
            "architecture", "flow", "diagram", "lifecycle", "pipeline", "monorepo",
            "hexagonal", "design", "component", "subsystem", "arquitetura", "fluxo",
            "camada", "visão geral", "master plan", "roadmap", "rfc", "fsm"
        ];
        let code_keywords = [
            "fn ", "function", "impl", "struct", "class", "def ", "import", "package",
            "compile", "syntax", "refactor", "bug", "panic", "error[e", "cargo",
            "async", "tokio", "api", "endpoint", "código", "função", "método", "type "
        ];
        let doc_keywords = [
            "invoice", "contract", "legal", "policy", "clause", "tax", "report", "cmr",
            "shipment", "receipt", "fatura", "contrato", "política", "relatório", "tributo",
            "imposto", "prazo", "nota fiscal", "termo"
        ];

        let mut sec_hits = 0;
        let mut devops_hits = 0;
        let mut data_hits = 0;
        let mut arch_hits = 0;
        let mut code_hits = 0;
        let mut doc_hits = 0;

        for kw in sec_keywords { if q.contains(kw) { sec_hits += 1; } }
        for kw in devops_keywords { if q.contains(kw) { devops_hits += 1; } }
        for kw in data_keywords { if q.contains(kw) { data_hits += 1; } }
        for kw in arch_keywords { if q.contains(kw) { arch_hits += 1; } }
        for kw in code_keywords { if q.contains(kw) { code_hits += 1; } }
        for kw in doc_keywords { if q.contains(kw) { doc_hits += 1; } }

        if sec_hits > 0 {
            (OnnxIntent::SecurityAudit, 0.45 + (sec_hits as f32 * 0.15).min(0.50))
        } else if devops_hits > 0 && devops_hits >= code_hits {
            (OnnxIntent::DevOpsInfra, 0.40 + (devops_hits as f32 * 0.15).min(0.50))
        } else if data_hits > 0 && data_hits >= code_hits {
            (OnnxIntent::DataSchema, 0.40 + (data_hits as f32 * 0.15).min(0.50))
        } else if arch_hits > 0 && arch_hits >= code_hits {
            (OnnxIntent::Architecture, 0.40 + (arch_hits as f32 * 0.15).min(0.55))
        } else if code_hits > 0 {
            (OnnxIntent::Code, 0.35 + (code_hits as f32 * 0.15).min(0.55))
        } else if doc_hits > 0 {
            (OnnxIntent::Document, 0.35 + (doc_hits as f32 * 0.15).min(0.55))
        } else {
            (OnnxIntent::General, 0.20)
        }
    }

    fn evaluate_fallback_complexity(
        q: &str,
        word_count: usize,
        intent: OnnxIntent,
    ) -> (OnnxComplexity, f32, String) {
        if word_count >= 20 || q.contains("compare") || q.contains("comparar") || intent == OnnxIntent::Architecture {
            (
                OnnxComplexity::Deep,
                0.75,
                "🧠 [Heuristic Router] Multi-clause pattern detected (<1µs)".to_string(),
            )
        } else {
            (
                OnnxComplexity::Fast,
                0.80,
                "⚡ [Heuristic Router] Low-latency direct lookup (<1µs)".to_string(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onnx_query_classifier_fallback() {
        let classifier = OnnxQueryClassifier::from_session(None);
        assert!(!classifier.is_neural_active());

        let (comp, _intent, _, reason) = classifier.classify_query("where is the timeout configuration?");
        assert_eq!(comp, OnnxComplexity::Fast);
        assert!(reason.contains("Heuristic Router"));

        let (deep_comp, _, _, _) = classifier.classify_query("compare the architecture of token_budget and bm25 step by step with pros and cons");
        assert_eq!(deep_comp, OnnxComplexity::Deep);
    }
}
