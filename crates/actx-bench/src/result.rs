//! The run result file: configuration, metrics, latency and per-question results.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::metrics::{Latency, MetricSet, QuestionResult};

pub const SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Retrievers {
    pub lexical: bool,
    pub dense: bool,
    pub reranker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunConfig {
    pub config_id: String,
    pub corpus_version: String,
    pub questions_version: String,
    pub retrievers: Retrievers,
    pub embedding_target: String,
    pub reranker_model: String,
    pub code_revision: String,
}

impl RunConfig {
    /// Fields that differ from `other` among those that make two runs comparable.
    pub fn differences(&self, other: &RunConfig) -> Vec<&'static str> {
        let mut diff = Vec::new();
        if self.config_id != other.config_id {
            diff.push("config_id");
        }
        if self.corpus_version != other.corpus_version {
            diff.push("corpus_version");
        }
        if self.questions_version != other.questions_version {
            diff.push("questions_version");
        }
        if self.retrievers != other.retrievers {
            diff.push("retrievers");
        }
        if self.embedding_target != other.embedding_target {
            diff.push("embedding_target");
        }
        if self.reranker_model != other.reranker_model {
            diff.push("reranker_model");
        }
        diff
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunResult {
    pub schema: u32,
    pub config: RunConfig,
    pub metrics: MetricSet,
    pub latency_ms: Latency,
    pub questions: Vec<QuestionResult>,
}

impl RunResult {
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("'{}' cannot be read: {e}", path.display()))?;
        let result: Self = serde_json::from_str(&text).map_err(|e| format!("'{}' is not a result file: {e}", path.display()))?;
        if result.schema != SCHEMA {
            return Err(format!("'{}' has schema {}, expected {SCHEMA}", path.display(), result.schema));
        }
        Ok(result)
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("'{}' cannot be created: {e}", parent.display()))?;
        }
        let mut text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        text.push('\n');
        std::fs::write(path, text).map_err(|e| format!("'{}' cannot be written: {e}", path.display()))
    }

    /// The same run without the fields that may vary between identical runs (latency, revision).
    pub fn quality_only(&self) -> Self {
        let mut copy = self.clone();
        copy.latency_ms = Latency::default();
        copy.config.code_revision = String::new();
        copy
    }
}
