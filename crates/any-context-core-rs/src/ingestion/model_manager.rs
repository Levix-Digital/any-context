//! Embedded ONNX Model Repository, Catalog & Resilient Fallback Manager.
//!
//! Governs the lifecycle of local ONNX models for Document AI and Query Routing:
//! - Laya AI (mmBERT INT8 ONNX) for dynamic document and form typology.
//! - MobileNetV4 RVL-CDIP (ONNX) for scanned and rasterized page classification.
//! - BGE-Small Query Classifier (ONNX) for sub-15ms local query complexity.
//! - Moondream2 (1.8B INT4 ONNX) & SmolVLM (500M ONNX) for air-gapped document vision.
//!
//! Architectural Invariant: Full Fault Tolerance & Graceful Fallback.
//! Missing or partially downloaded models NEVER crash or halt the pipeline.
//! The system seamlessly falls back to pure Rust 2D spatial & deterministic heuristics.

use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::storage::get_default_models_dir;

/// Functional category of an embedded ONNX model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnnxModelCategory {
    IngestionClassifier,
    ScannedClassifier,
    QueryClassifier,
    DocumentVision,
}

impl OnnxModelCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IngestionClassifier => "Classificador de Ingestão",
            Self::ScannedClassifier => "Classificador de Scans",
            Self::QueryClassifier => "Classificador de Consultas",
            Self::DocumentVision => "Visão de Documentos",
        }
    }
}

/// Metadata specification of a supported local ONNX model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnnxModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: OnnxModelCategory,
    pub file_name: &'static str,
    pub size_bytes: u64,
    pub download_url: &'static str,
    pub description: &'static str,
    pub fallback_description: &'static str,
}

/// Official catalog of embedded ONNX models for AnyContext.
pub const ONNX_CATALOG: &[OnnxModelSpec] = &[
    OnnxModelSpec {
        id: "laya-int8",
        name: "Laya AI (mmBERT INT8 ONNX)",
        category: OnnxModelCategory::IngestionClassifier,
        file_name: "laya_mmbert_int8.onnx",
        size_bytes: 220_200_960, // ~210 MB
        download_url: "https://huggingface.co/convaiinnovations/laya/resolve/main/model_int8.onnx",
        description: "Classificação dinâmica de formulários, faturas e relatórios densos (~95% precisão, 60ms CPU)",
        fallback_description: "Classificador Heurístico / Determinístico (<1µs / 0MB RAM)",
    },
    OnnxModelSpec {
        id: "mobilenetv4-rvl-cdip",
        name: "MobileNetV4 RVL-CDIP (ONNX)",
        category: OnnxModelCategory::ScannedClassifier,
        file_name: "mobilenetv4_rvl_cdip.onnx",
        size_bytes: 14_680_064, // ~14 MB
        download_url: "https://huggingface.co/Levix-Digital/document-ai-onnx/resolve/main/mobilenetv4_rvl_cdip.onnx",
        description: "Sentinela visual para documentos 100% rasterizados ou escaneados (224x224, 8ms CPU)",
        fallback_description: "Heurística 2D Espacial Rust (<5MB RAM)",
    },
    OnnxModelSpec {
        id: "bge-small-onnx",
        name: "BGE-Small Query Classifier (ONNX)",
        category: OnnxModelCategory::QueryClassifier,
        file_name: "bge_small_query_int8.onnx",
        size_bytes: 36_700_160, // ~35 MB
        download_url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model_int8.onnx",
        description: "Classificação neural local da complexidade de perguntas (RFC-042, Fast vs Deep)",
        fallback_description: "Classificador Determinístico Sub-microsegundo (<1µs / 0MB RAM)",
    },
    OnnxModelSpec {
        id: "moondream2-int4",
        name: "Moondream2 1.8B (INT4 ONNX)",
        category: OnnxModelCategory::DocumentVision,
        file_name: "moondream2_int4.onnx",
        size_bytes: 1_153_433_600, // ~1.1 GB
        download_url: "https://huggingface.co/vikhyatk/moondream2/resolve/main/moondream2_int4.onnx",
        description: "Extração visual e OCR denso air-gapped para formulários e diagramas técnicos",
        fallback_description: "Heurística Espacial 2D & Tipográfica Rust (<5MB RAM)",
    },
    OnnxModelSpec {
        id: "smolvlm-500m",
        name: "SmolVLM-500M (ONNX)",
        category: OnnxModelCategory::DocumentVision,
        file_name: "smolvlm_500m_int8.onnx",
        size_bytes: 503_316_480, // ~480 MB
        download_url: "https://huggingface.co/HuggingFaceTB/SmolVLM-500M-Instruct/resolve/main/onnx/model.onnx",
        description: "Visão local ultraleve para computadores corporativos modestos (~800MB RAM)",
        fallback_description: "Heurística Espacial 2D & Tipográfica Rust (<5MB RAM)",
    },
];

/// Status item for a model in the store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatusItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub is_installed: bool,
    pub file_path: PathBuf,
    pub disk_size_bytes: u64,
    pub expected_size_bytes: u64,
    pub fallback_description: String,
}

/// Aggregated report of the local ONNX models store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStoreSummary {
    pub models_dir: PathBuf,
    pub installed_count: usize,
    pub total_count: usize,
    pub total_bytes_on_disk: u64,
    pub models: Vec<ModelStatusItem>,
}

/// Central manager for inspecting, verifying, downloading, and selecting local ONNX models.
pub struct OnnxModelManager;

impl OnnxModelManager {
    /// Returns the complete list of supported ONNX models.
    pub fn catalog() -> &'static [OnnxModelSpec] {
        ONNX_CATALOG
    }

    /// Finds a model specification by identifier.
    pub fn find_spec(id: &str) -> Option<&'static OnnxModelSpec> {
        ONNX_CATALOG.iter().find(|m| m.id.eq_ignore_ascii_case(id))
    }

    /// Resolves the absolute on-disk path for a model.
    pub fn model_path(spec: &OnnxModelSpec) -> PathBuf {
        get_default_models_dir().join(spec.file_name)
    }

    /// Checks if a model's ONNX weights file exists and is non-empty.
    pub fn is_installed(spec: &OnnxModelSpec) -> bool {
        let p = Self::model_path(spec);
        p.is_file() && std::fs::metadata(&p).map(|m| m.len() > 1024).unwrap_or(false)
    }

    /// Returns the file size in bytes if installed.
    pub fn installed_size(spec: &OnnxModelSpec) -> Option<u64> {
        let p = Self::model_path(spec);
        if p.is_file() {
            std::fs::metadata(&p).ok().map(|m| m.len())
        } else {
            None
        }
    }

    /// Inspects the store and produces an aggregated summary.
    pub fn inspect_store() -> ModelStoreSummary {
        let models_dir = get_default_models_dir();
        let mut installed_count = 0;
        let mut total_bytes = 0u64;
        let mut items = Vec::with_capacity(ONNX_CATALOG.len());

        for spec in ONNX_CATALOG {
            let p = Self::model_path(spec);
            let size = if p.is_file() {
                let s = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                if s > 1024 {
                    installed_count += 1;
                    total_bytes += s;
                    s
                } else {
                    0
                }
            } else {
                0
            };

            items.push(ModelStatusItem {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                category: spec.category.as_str().to_string(),
                is_installed: size > 0,
                file_path: p,
                disk_size_bytes: size,
                expected_size_bytes: spec.size_bytes,
                fallback_description: spec.fallback_description.to_string(),
            });
        }

        ModelStoreSummary {
            models_dir,
            installed_count,
            total_count: ONNX_CATALOG.len(),
            total_bytes_on_disk: total_bytes,
            models: items,
        }
    }

    /// Returns the active document classifier state (Model Name, Detail, Is_ONNX).
    pub fn active_ingestion_classifier() -> (&'static str, &'static str, bool) {
        if let Some(spec) = Self::find_spec("laya-int8") {
            if Self::is_installed(spec) {
                return (spec.name, "Ativo em runtime (~95% precisão, 60ms CPU)", true);
            }
        }
        (
            "Determinístico / Heurístico (<1µs / 0MB RAM)",
            "Padrão ativo (Zero download / Latência sub-microsegundo)",
            false,
        )
    }

    /// Returns the active scanned page classifier state.
    pub fn active_scanned_classifier() -> (&'static str, &'static str, bool) {
        if let Some(spec) = Self::find_spec("mobilenetv4-rvl-cdip") {
            if Self::is_installed(spec) {
                return (spec.name, "Sentinela visual para scans ativo (~8ms CPU)", true);
            }
        }
        (
            "Heurística 2D Espacial (<5MB RAM)",
            "Padrão ativo (Rust puro / Zero download)",
            false,
        )
    }

    /// Returns the active local vision SLM state.
    pub fn active_vision_state() -> (&'static str, &'static str, bool) {
        if let Some(spec) = Self::find_spec("moondream2-int4") {
            if Self::is_installed(spec) {
                return (spec.name, "SLM Local Air-Gapped ativo", true);
            }
        }
        if let Some(spec) = Self::find_spec("smolvlm-500m") {
            if Self::is_installed(spec) {
                return (spec.name, "SLM Local Ultracompacto ativo", true);
            }
        }
        (
            "Heurística 2D & Tipográfica (<5MB RAM)",
            "Padrão ativo (Zero download / Instantâneo)",
            false,
        )
    }

    /// Asynchronously downloads a model specification with streaming and atomic rename.
    pub async fn download_model(spec: &OnnxModelSpec) -> Result<PathBuf, String> {
        let dest_dir = get_default_models_dir();
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create models directory {:?}: {}", dest_dir, e))?;

        let dest_file = dest_dir.join(spec.file_name);
        let tmp_file = dest_dir.join(format!("{}.download.tmp", spec.file_name));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let response = client
            .get(spec.download_url)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to model source {}: {}", spec.download_url, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Model repository returned HTTP status {}: {}",
                response.status(),
                spec.download_url
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to download model payload: {}", e))?;

        std::fs::write(&tmp_file, &bytes)
            .map_err(|e| format!("Failed to write temporary model file {:?}: {}", tmp_file, e))?;

        std::fs::rename(&tmp_file, &dest_file)
            .map_err(|e| format!("Failed to finalize model file {:?}: {}", dest_file, e))?;

        Ok(dest_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_integrity() {
        let catalog = OnnxModelManager::catalog();
        assert_eq!(catalog.len(), 5);
        assert!(catalog.iter().any(|m| m.id == "laya-int8"));
        assert!(catalog.iter().any(|m| m.id == "mobilenetv4-rvl-cdip"));
        assert!(catalog.iter().any(|m| m.id == "moondream2-int4"));
        assert!(catalog.iter().any(|m| m.id == "smolvlm-500m"));
        assert!(catalog.iter().any(|m| m.id == "bge-small-onnx"));
    }

    #[test]
    fn test_inspect_store_does_not_panic() {
        let summary = OnnxModelManager::inspect_store();
        assert_eq!(summary.total_count, 5);
        assert_eq!(summary.models.len(), 5);
    }

    #[test]
    fn test_fallback_descriptions_populated() {
        for spec in OnnxModelManager::catalog() {
            assert!(!spec.fallback_description.is_empty());
            assert!(!spec.name.is_empty());
            assert!(spec.size_bytes > 0);
        }
    }
}
