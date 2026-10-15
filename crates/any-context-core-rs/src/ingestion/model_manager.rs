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
    LanguageModel,
    IngestionClassifier,
    ScannedClassifier,
    QueryClassifier,
    DocumentVision,
}

impl OnnxModelCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LanguageModel => "Local Language Model",
            Self::IngestionClassifier => "Ingestion Classifier",
            Self::ScannedClassifier => "Scan Classifier",
            Self::QueryClassifier => "Query Classifier",
            Self::DocumentVision => "Document Vision",
        }
    }
}

/// Metadata specification of a supported local ONNX model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    // --- Gemma 4 Sovereign Local Brain Family (INT4/INT8 Quantized ONNX) ---
    OnnxModelSpec {
        id: "gemma-4-compact",
        name: "Gemma 4 Compact (INT4 ONNX)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_compact_int8.onnx",
        size_bytes: 1_850_000_000, // ~1.85 GB
        download_url: "https://huggingface.co/onnx-community/gemma-4-compact-ONNX/resolve/main/onnx/model_int8.onnx",
        description: "Sovereign Edge Mobility Language Model (Edge / Laptop, ~2.5GB Peak RAM)",
        fallback_description: "Mock Offline Provider ($0.00 / Zero Network Leaks)",
    },
    OnnxModelSpec {
        id: "gemma-4-12b",
        name: "Gemma 4 12B (INT4 ONNX)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_12b_int8.onnx",
        size_bytes: 6_750_000_000, // ~6.75 GB
        download_url: "https://huggingface.co/onnx-community/gemma-4-12b-ONNX/resolve/main/onnx/model_int8.onnx",
        description: "Developer Sweet Spot Local Language Model (Balanced Workstation, ~10.0GB Peak RAM)",
        fallback_description: "Mock Offline Provider ($0.00 / Zero Network Leaks)",
    },
    OnnxModelSpec {
        id: "gemma-4-27b",
        name: "Gemma 4 27B (INT4 ONNX)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_27b_int8.onnx",
        size_bytes: 14_800_000_000, // ~14.8 GB
        download_url: "https://huggingface.co/onnx-community/gemma-4-27b-ONNX/resolve/main/onnx/model_int8.onnx",
        description: "Workstation High-Reasoning Local Language Model (Advanced Workstation, ~18.5GB Peak RAM)",
        fallback_description: "Mock Offline Provider ($0.00 / Zero Network Leaks)",
    },
    OnnxModelSpec {
        id: "gemma-4-32b",
        name: "Gemma 4 32B (INT4 ONNX)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_32b_int8.onnx",
        size_bytes: 20_200_000_000, // ~20.2 GB
        download_url: "https://huggingface.co/onnx-community/gemma-4-32b-ONNX/resolve/main/onnx/model_int8.onnx",
        description: "Sovereign Frontier Local Language Model (High-End Rig / Server, ~25.5GB Peak RAM)",
        fallback_description: "Mock Offline Provider ($0.00 / Zero Network Leaks)",
    },
    // --- Document AI & Query Routing Models ---
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

/// Real-time calculated metrics for download progress presentation across all surfaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgressInfo {
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub percentage: f32,
    pub bytes_per_sec: f64,
    pub eta_seconds: Option<u64>,
}

impl DownloadProgressInfo {
    /// Computes percentage, transfer speed, and ETA from raw bytes and elapsed duration.
    pub fn compute(downloaded: u64, total: u64, elapsed: std::time::Duration) -> Self {
        let percentage = if total > 0 {
            ((downloaded as f64 / total as f64) * 100.0).clamp(0.0, 100.0) as f32
        } else {
            0.0
        };
        let secs = elapsed.as_secs_f64();
        let bytes_per_sec = if secs > 0.05 {
            downloaded as f64 / secs
        } else {
            0.0
        };
        let eta_seconds = if bytes_per_sec > 1024.0 && total > downloaded {
            let remaining = total - downloaded;
            Some((remaining as f64 / bytes_per_sec) as u64)
        } else {
            None
        };
        Self {
            downloaded_bytes: downloaded,
            total_bytes: total,
            percentage,
            bytes_per_sec,
            eta_seconds,
        }
    }

    pub fn downloaded_mb(&self) -> f64 {
        self.downloaded_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn total_mb(&self) -> f64 {
        self.total_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn formatted_speed(&self) -> String {
        let mb_s = self.bytes_per_sec / (1024.0 * 1024.0);
        if mb_s >= 1.0 {
            format!("{:.1} MB/s", mb_s)
        } else {
            let kb_s = self.bytes_per_sec / 1024.0;
            format!("{:.0} KB/s", kb_s)
        }
    }

    pub fn formatted_eta(&self) -> String {
        match self.eta_seconds {
            Some(secs) if secs < 60 => format!("{}s", secs),
            Some(secs) => format!("{}m {:02}s", secs / 60, secs % 60),
            None => "--".to_string(),
        }
    }
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

    /// Resolves a user-provided tier name or alias into a canonical ONNX model specification.
    ///
    /// Supported aliases:
    /// - "compact", "gemma-compact", "2b", "3b" -> "gemma-4-compact"
    /// - "12b", "gemma-12b" -> "gemma-4-12b"
    /// - "27b", "gemma-27b" -> "gemma-4-27b"
    /// - "32b", "gemma-32b" -> "gemma-4-32b"
    pub fn resolve_spec(query: &str) -> Option<&'static OnnxModelSpec> {
        let clean = query.trim().to_lowercase();
        if clean.is_empty() {
            return None;
        }

        // Direct ID or filename match
        if let Some(spec) = Self::find_spec(&clean) {
            return Some(spec);
        }
        for spec in ONNX_CATALOG {
            if spec.file_name.eq_ignore_ascii_case(&clean) {
                return Some(spec);
            }
        }

        // Flexible alias resolution
        if clean == "compact" || clean == "gemma-compact" || clean == "2b" || clean == "3b" || clean.contains("compact") {
            Self::find_spec("gemma-4-compact")
        } else if clean == "12b" || clean == "gemma-12b" || clean.contains("12b") || clean.contains("12") {
            Self::find_spec("gemma-4-12b")
        } else if clean == "27b" || clean == "gemma-27b" || clean.contains("27b") || clean.contains("27") {
            Self::find_spec("gemma-4-27b")
        } else if clean == "32b" || clean == "gemma-32b" || clean.contains("32b") || clean.contains("32") {
            Self::find_spec("gemma-4-32b")
        } else if clean == "laya" {
            Self::find_spec("laya-int8")
        } else if clean == "mobilenet" || clean == "scans" {
            Self::find_spec("mobilenetv4-rvl-cdip")
        } else if clean == "bge" || clean == "query" {
            Self::find_spec("bge-small-onnx")
        } else if clean == "layoutlm" || clean == "layout" {
            Self::find_spec("layoutlmv3-int8")
        } else if clean == "clip" || clean == "vision" {
            Self::find_spec("clip-vit-int8")
        } else {
            None
        }
    }

    /// Validates disk space and hardware RAM limits before commencing model weights download.
    /// Returns Ok(()) if safe, or an informative Err explaining the exact safety or disk violation.
    pub fn check_download_safety(spec: &OnnxModelSpec, force: bool) -> Result<(), String> {
        let metrics = crate::hardware::HardwareProfiler::probe_system(None);

        // 1. Check free disk space with a 500 MB buffer
        let buffer_bytes = 500 * 1024 * 1024u64;
        let required_disk = spec.size_bytes.saturating_add(buffer_bytes);
        if metrics.free_disk_bytes < required_disk {
            let req_gb = required_disk as f64 / (1024.0 * 1024.0 * 1024.0);
            let free_gb = metrics.free_disk_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            return Err(format!(
                "Insufficient disk space in models directory '{:?}': requires {:.2} GB free (including 500MB safety buffer), but only {:.2} GB is available.",
                metrics.target_drive_path, req_gb, free_gb
            ));
        }

        // 2. Check RAM safety for Language Models (Gemma 4 family)
        if spec.category == OnnxModelCategory::LanguageModel {
            let verdict = crate::hardware::HardwareProfiler::validate_model_safety(spec.id, &metrics);
            if let crate::hardware::HardwareSafetyVerdict::Unsafe {
                required_ram_gb,
                total_ram_gb,
                recommended_model,
                ..
            } = verdict {
                if !force {
                    return Err(format!(
                        "❌ [Hardware Warning - LM Studio Style]: Model '{}' requires ~{:.1} GB of physical RAM, but system has only {:.1} GB. Downloading and running this model will cause severe swap thrashing, system freeze, or OOM. Recommended local model: '{}'. Pass '--force' if you explicitly choose to proceed.",
                        spec.name, required_ram_gb, total_ram_gb, recommended_model
                    ));
                }
            }
        }

        Ok(())
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
        assert_eq!(catalog.len(), 9);
        assert!(catalog.iter().any(|m| m.id == "gemma-4-compact"));
        assert!(catalog.iter().any(|m| m.id == "gemma-4-12b"));
        assert!(catalog.iter().any(|m| m.id == "gemma-4-27b"));
        assert!(catalog.iter().any(|m| m.id == "gemma-4-32b"));
        assert!(catalog.iter().any(|m| m.id == "laya-int8"));
        assert!(catalog.iter().any(|m| m.id == "mobilenetv4-rvl-cdip"));
        assert!(catalog.iter().any(|m| m.id == "layoutlmv3-int8"));
        assert!(catalog.iter().any(|m| m.id == "clip-vit-int8"));
        assert!(catalog.iter().any(|m| m.id == "bge-small-onnx"));
    }

    #[test]
    fn test_inspect_store_does_not_panic() {
        let summary = OnnxModelManager::inspect_store();
        assert_eq!(summary.total_count, 9);
        assert_eq!(summary.models.len(), 9);
    }

    #[test]
    fn test_fallback_descriptions_populated() {
        for spec in OnnxModelManager::catalog() {
            assert!(!spec.fallback_description.is_empty());
            assert!(!spec.name.is_empty());
            assert!(spec.size_bytes > 0);
        }
    }

    #[test]
    fn test_resolve_spec_aliases() {
        assert_eq!(OnnxModelManager::resolve_spec("compact").unwrap().id, "gemma-4-compact");
        assert_eq!(OnnxModelManager::resolve_spec("12b").unwrap().id, "gemma-4-12b");
        assert_eq!(OnnxModelManager::resolve_spec("gemma-4-27b").unwrap().id, "gemma-4-27b");
        assert_eq!(OnnxModelManager::resolve_spec("32b").unwrap().id, "gemma-4-32b");
        assert_eq!(OnnxModelManager::resolve_spec("laya").unwrap().id, "laya-int8");
        assert_eq!(OnnxModelManager::resolve_spec("scans").unwrap().id, "mobilenetv4-rvl-cdip");
        assert_eq!(OnnxModelManager::resolve_spec("unknown-nonexistent"), None);
    }

    #[test]
    fn test_download_progress_info_compute() {
        let info = DownloadProgressInfo::compute(
            500_000_000,
            1_000_000_000,
            std::time::Duration::from_secs(10),
        );
        assert_eq!(info.percentage, 50.0);
        assert!(info.bytes_per_sec > 45_000_000.0);
        assert!(info.eta_seconds.is_some());
        assert_eq!(info.formatted_speed(), "47.7 MB/s");
        assert_eq!(info.formatted_eta(), "10s");
    }
}
