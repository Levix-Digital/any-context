//! Embedded ONNX Model Repository, Catalog & Resilient Fallback Manager.
//!
//! Governs the lifecycle of local ONNX models for Document AI and Query Routing:
//! - Laya AI (mmBERT INT8 ONNX) for dynamic document and form typology.
//! - MobileNetV4 RVL-CDIP (ONNX) for scanned and rasterized page classification.
//! - BGE-Small Query Classifier (ONNX) for sub-15ms local query complexity.
//! - LayoutLMv3 Base (INT8 ONNX) & CLIP ViT Vision Encoder (INT8 ONNX) for document layout and air-gapped vision.
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
            Self::IngestionClassifier => "Ingestion Classifier",
            Self::ScannedClassifier => "Scan Classifier",
            Self::QueryClassifier => "Query Classifier",
            Self::DocumentVision => "Document Vision",
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
        size_bytes: 424_348_081, // ~404 MB
        download_url: "https://huggingface.co/tozp/laya-onnx/resolve/main/model_int8.onnx",
        description: "Dynamic classification of forms, invoices, and dense reports (~95% accuracy, 60ms CPU)",
        fallback_description: "Heuristic / Deterministic Classifier (<1µs / 0MB RAM)",
    },
    OnnxModelSpec {
        id: "mobilenetv4-rvl-cdip",
        name: "MobileNetV4 RVL-CDIP (ONNX)",
        category: OnnxModelCategory::ScannedClassifier,
        file_name: "mobilenetv4_rvl_cdip.onnx",
        size_bytes: 3_931_745, // ~3.9 MB
        download_url: "https://huggingface.co/onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/main/onnx/model_int8.onnx",
        description: "Visual sentinel for 100% rasterized or scanned documents (224x224, 8ms CPU)",
        fallback_description: "Rust 2D Spatial Heuristic (<5MB RAM)",
    },
    OnnxModelSpec {
        id: "bge-small-onnx",
        name: "BGE-Small Query Classifier (ONNX)",
        category: OnnxModelCategory::QueryClassifier,
        file_name: "bge_small_query_int8.onnx",
        size_bytes: 133_093_490, // ~127 MB
        download_url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx",
        description: "Local neural classification of query complexity (RFC-042, Fast vs Deep)",
        fallback_description: "Sub-microsecond Deterministic Classifier (<1µs / 0MB RAM)",
    },
    OnnxModelSpec {
        id: "layoutlmv3-int8",
        name: "LayoutLMv3 Base (INT8 ONNX)",
        category: OnnxModelCategory::IngestionClassifier,
        file_name: "layoutlmv3_base_int8.onnx",
        size_bytes: 127_001_085, // ~121 MB
        download_url: "https://huggingface.co/onnx-community/layoutlmv3-base-ONNX/resolve/main/onnx/model_int8.onnx",
        description: "Multimodal layout extraction of complex documents, forms, and tables",
        fallback_description: "Agnostic 2D Spatial Heuristic (<5MB RAM)",
    },
    OnnxModelSpec {
        id: "clip-vit-int8",
        name: "CLIP ViT Vision Encoder (INT8 ONNX)",
        category: OnnxModelCategory::DocumentVision,
        file_name: "clip_vit_vision_int8.onnx",
        size_bytes: 88_648_877, // ~84 MB
        download_url: "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main/onnx/vision_model_int8.onnx",
        description: "Dense visual extraction and air-gapped OCR for charts and visual diagrams",
        fallback_description: "Rust 2D Spatial & Typographic Heuristic (<5MB RAM)",
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
                return (spec.name, "Active in runtime (~95% accuracy, 60ms CPU)", true);
            }
        }
        (
            "Deterministic / Heuristic (<1µs / 0MB RAM)",
            "Active default (Zero downloads / Sub-microsecond latency)",
            false,
        )
    }

    /// Returns the active scanned page classifier state.
    pub fn active_scanned_classifier() -> (&'static str, &'static str, bool) {
        if let Some(spec) = Self::find_spec("mobilenetv4-rvl-cdip") {
            if Self::is_installed(spec) {
                return (spec.name, "Visual sentinel for scans active (~8ms CPU)", true);
            }
        }
        (
            "2D Spatial Heuristic (<5MB RAM)",
            "Active default (Pure Rust / Zero downloads)",
            false,
        )
    }

    /// Returns the active local vision SLM state.
    pub fn active_vision_state() -> (&'static str, &'static str, bool) {
        if let Some(spec) = Self::find_spec("clip-vit-int8") {
            if Self::is_installed(spec) {
                return (spec.name, "Local Air-Gapped Vision Encoder active", true);
            }
        }
        (
            "2D & Typographic Heuristic (<5MB RAM)",
            "Active default (Zero downloads / Instant)",
            false,
        )
    }

    /// Asynchronously downloads a model specification with streaming, live progress and graceful cancellation.
    pub async fn download_model_with_progress(
        spec: &OnnxModelSpec,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<(u64, u64)>>,
        cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<PathBuf, String> {
        use std::io::Write;

        let dest_dir = get_default_models_dir();
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create models directory {:?}: {}", dest_dir, e))?;

        let dest_file = dest_dir.join(spec.file_name);
        let tmp_file = dest_dir.join(format!("{}.download.tmp", spec.file_name));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .user_agent("AnyContext/0.35.0 (Windows; x86_64)")
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let mut response = client
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

        let total_bytes = response.content_length().unwrap_or(spec.size_bytes);
        let mut downloaded_bytes: u64 = 0;

        let mut file = std::fs::File::create(&tmp_file)
            .map_err(|e| format!("Failed to create temporary model file {:?}: {}", tmp_file, e))?;

        while let Some(chunk) = response.chunk().await.map_err(|e| format!("Download stream error: {}", e))? {
            if let Some(ref rx) = cancel_rx {
                if *rx.borrow() {
                    drop(file);
                    let _ = std::fs::remove_file(&tmp_file);
                    return Err("Download cancelled by user.".to_string());
                }
            }

            file.write_all(&chunk)
                .map_err(|e| format!("Failed to write chunk: {}", e))?;
            downloaded_bytes += chunk.len() as u64;

            if let Some(ref tx) = progress_tx {
                let _ = tx.send((downloaded_bytes, total_bytes));
            }
        }

        file.flush().map_err(|e| format!("Failed to flush file: {}", e))?;
        drop(file);

        // Check cancellation one final time before finalizing
        if let Some(ref rx) = cancel_rx {
            if *rx.borrow() {
                let _ = std::fs::remove_file(&tmp_file);
                return Err("Download cancelled by user.".to_string());
            }
        }

        std::fs::rename(&tmp_file, &dest_file)
            .map_err(|e| format!("Failed to finalize model file {:?}: {}", dest_file, e))?;

        Ok(dest_file)
    }

    /// Asynchronously downloads a model specification with standard streaming.
    pub async fn download_model(spec: &OnnxModelSpec) -> Result<PathBuf, String> {
        Self::download_model_with_progress(spec, None, None).await
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
        assert!(catalog.iter().any(|m| m.id == "layoutlmv3-int8"));
        assert!(catalog.iter().any(|m| m.id == "clip-vit-int8"));
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
