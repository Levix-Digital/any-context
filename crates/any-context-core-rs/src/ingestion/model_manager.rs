//! Local ONNX Model Repository, Verified Catalog & Resilient Fallback Manager (ADR-118 / ADR-130).
//!
//! Governs the lifecycle of downloadable local models:
//! - Cross-encoder re-rankers (neural, executed by ONNX Runtime on CPU).
//! - Document AI & query-routing weights (downloadable; their components run deterministic
//!   heuristics until neural inference is implemented).
//! - Gemma 4 local generation (planned; no downloadable artifact yet).
//!
//! Every downloadable file is pinned to an immutable upstream revision and verified by
//! size and SHA-256 before it is atomically moved into place.
//!
//! Architectural Invariant: Full Fault Tolerance & Graceful Fallback (ADR-110).
//! Missing or partially downloaded models NEVER crash or halt the pipeline.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::storage::get_default_models_dir;

/// Functional category of a local model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OnnxModelCategory {
    LanguageModel,
    IngestionClassifier,
    ScannedClassifier,
    QueryClassifier,
    DocumentVision,
    ReRanker,
    Embedding,
}

impl OnnxModelCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LanguageModel => "Local Language Model",
            Self::IngestionClassifier => "Ingestion Classifier",
            Self::ScannedClassifier => "Scan Classifier",
            Self::QueryClassifier => "Query Classifier",
            Self::DocumentVision => "Document Vision",
            Self::ReRanker => "Neural Re-Ranker",
            Self::Embedding => "Embedding Model",
        }
    }
}

/// Whether a catalog entry can be downloaded and used in this build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ModelAvailability {
    /// Verified files exist upstream and can be installed.
    Available,
    /// Announced capability without a usable artifact yet (no download).
    Planned,
}

/// A single verified file belonging to a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelFile {
    /// Path relative to the models directory (`/`-separated).
    pub file_name: &'static str,
    /// Upstream URL pinned to an immutable revision.
    pub url: &'static str,
    pub size_bytes: u64,
    /// Lowercase hex SHA-256 of the file.
    pub sha256: &'static str,
}

/// Metadata specification of a supported local model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OnnxModelSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub category: OnnxModelCategory,
    /// Primary file (relative to the models directory) used to locate the model.
    pub file_name: &'static str,
    /// Total size of all files in bytes (0 for planned entries).
    pub size_bytes: u64,
    pub files: &'static [ModelFile],
    pub availability: ModelAvailability,
    pub description: &'static str,
    pub fallback_description: &'static str,
}

impl OnnxModelSpec {
    pub fn is_available(&self) -> bool {
        self.availability == ModelAvailability::Available
    }
}

/// Environment override of the model mirror base URL.
pub const MODELS_MIRROR_ENV: &str = "ACTX_MODELS_MIRROR";
/// Setting with the model mirror base URL (an internal server that serves the catalog's files
/// under their `file_name` paths).
pub const MODELS_MIRROR_SETTING: &str = "models_mirror";

/// Default cross-encoder used by the two-stage retrieval pipeline.
pub const DEFAULT_RERANKER_ID: &str = "mmarco-reranker";

/// Official catalog of local models for AnyContext.
pub const ONNX_CATALOG: &[OnnxModelSpec] = &[
    // --- Gemma 4 local generation (planned: no generative runtime yet, see ADR-130) ---
    OnnxModelSpec {
        id: "gemma-4-compact",
        name: "Gemma 4 Compact (Planned)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_compact_int8.onnx",
        size_bytes: 0,
        files: &[],
        availability: ModelAvailability::Planned,
        description: "Local generation planned for a future release (no download available yet)",
        fallback_description: "Configured fallback provider (cloud, Ollama, LM Studio or Mock)",
    },
    OnnxModelSpec {
        id: "gemma-4-12b",
        name: "Gemma 4 12B (Planned)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_12b_int8.onnx",
        size_bytes: 0,
        files: &[],
        availability: ModelAvailability::Planned,
        description: "Local generation planned for a future release (no download available yet)",
        fallback_description: "Configured fallback provider (cloud, Ollama, LM Studio or Mock)",
    },
    OnnxModelSpec {
        id: "gemma-4-27b",
        name: "Gemma 4 27B (Planned)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_27b_int8.onnx",
        size_bytes: 0,
        files: &[],
        availability: ModelAvailability::Planned,
        description: "Local generation planned for a future release (no download available yet)",
        fallback_description: "Configured fallback provider (cloud, Ollama, LM Studio or Mock)",
    },
    OnnxModelSpec {
        id: "gemma-4-32b",
        name: "Gemma 4 32B (Planned)",
        category: OnnxModelCategory::LanguageModel,
        file_name: "gemma_4_32b_int8.onnx",
        size_bytes: 0,
        files: &[],
        availability: ModelAvailability::Planned,
        description: "Local generation planned for a future release (no download available yet)",
        fallback_description: "Configured fallback provider (cloud, Ollama, LM Studio or Mock)",
    },
    // --- Document AI & Query Routing weights (components run deterministic heuristics) ---
    OnnxModelSpec {
        id: "laya-int8",
        name: "Laya AI (mmBERT INT8 ONNX)",
        category: OnnxModelCategory::IngestionClassifier,
        file_name: "laya_mmbert_int8.onnx",
        size_bytes: 424_348_081,
        files: &[ModelFile {
            file_name: "laya_mmbert_int8.onnx",
            url: "https://huggingface.co/tozp/laya-onnx/resolve/0d1f7ebf46a3ea04ec4424df602f96ddefb66766/model_int8.onnx",
            size_bytes: 424_348_081,
            sha256: "d337ce1b1cbca907a4063223517af6db7e89f5c9e8d6a2f6a289babc256f4469",
        }],
        availability: ModelAvailability::Available,
        description: "Document & form typology weights (neural classification planned; heuristic active)",
        fallback_description: "Heuristic / Deterministic Classifier (<1µs / 0MB RAM)",
    },
    OnnxModelSpec {
        id: "mobilenetv4-rvl-cdip",
        name: "MobileNetV4 RVL-CDIP (ONNX)",
        category: OnnxModelCategory::ScannedClassifier,
        file_name: "mobilenetv4_rvl_cdip.onnx",
        size_bytes: 3_931_745,
        files: &[ModelFile {
            file_name: "mobilenetv4_rvl_cdip.onnx",
            url: "https://huggingface.co/onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/e832d3b2b2fd988f41063559c6ea30df281f4e05/onnx/model_int8.onnx",
            size_bytes: 3_931_745,
            sha256: "9930547d7d61660d667b72069f0fa75b706b3e0b04645cf99f76416a82fa99a3",
        }],
        availability: ModelAvailability::Available,
        description: "Scanned-page sentinel weights (neural inference planned; 2D heuristic active)",
        fallback_description: "Rust 2D Spatial Heuristic (<5MB RAM)",
    },
    OnnxModelSpec {
        id: "bge-small-onnx",
        name: "BGE-Small EN v1.5 (ONNX)",
        category: OnnxModelCategory::QueryClassifier,
        file_name: "bge_small_query_int8.onnx",
        size_bytes: 133_093_490,
        files: &[ModelFile {
            file_name: "bge_small_query_int8.onnx",
            url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a/onnx/model.onnx",
            size_bytes: 133_093_490,
            sha256: "828e1496d7fabb79cfa4dcd84fa38625c0d3d21da474a00f08db0f559940cf35",
        }],
        availability: ModelAvailability::Available,
        description: "Query routing weights (neural routing planned; keyword heuristic active)",
        fallback_description: "Sub-microsecond Deterministic Classifier (<1µs / 0MB RAM)",
    },
    OnnxModelSpec {
        id: "layoutlmv3-int8",
        name: "LayoutLMv3 Base (INT8 ONNX)",
        category: OnnxModelCategory::IngestionClassifier,
        file_name: "layoutlmv3_base_int8.onnx",
        size_bytes: 127_001_085,
        files: &[ModelFile {
            file_name: "layoutlmv3_base_int8.onnx",
            url: "https://huggingface.co/onnx-community/layoutlmv3-base-ONNX/resolve/43d4a6517dc9b9a5eb3d538a158161ead8d028b9/onnx/model_int8.onnx",
            size_bytes: 127_001_085,
            sha256: "5f75612d2c2cd3b23826b7fcbed33e11f39d6acda1fe66fa091a0392b18a9013",
        }],
        availability: ModelAvailability::Available,
        description: "Layout extraction weights (neural inference planned; 2D spatial chunkers active)",
        fallback_description: "Agnostic 2D Spatial Heuristic (<5MB RAM)",
    },
    OnnxModelSpec {
        id: "clip-vit-int8",
        name: "CLIP ViT Vision Encoder (INT8 ONNX)",
        category: OnnxModelCategory::DocumentVision,
        file_name: "clip_vit_vision_int8.onnx",
        size_bytes: 88_648_877,
        files: &[ModelFile {
            file_name: "clip_vit_vision_int8.onnx",
            url: "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/d15189d7028b43f1d3e65039190477f6af591c2a/onnx/vision_model_int8.onnx",
            size_bytes: 88_648_877,
            sha256: "0ab0c1b3ace708e539633af1744d5a95247fe4e14d3e08ff197ef82a6cb9bd93",
        }],
        availability: ModelAvailability::Available,
        description: "Visual encoder weights (neural inference planned; native visual chunks preserved)",
        fallback_description: "Rust 2D Spatial & Typographic Heuristic (<5MB RAM)",
    },
    // --- Cross-encoder re-rankers (neural, ONNX Runtime CPU) ---
    OnnxModelSpec {
        id: "mmarco-reranker",
        name: "mmarco mMiniLMv2-L12 Multilingual Cross-Encoder (INT8 ONNX)",
        category: OnnxModelCategory::ReRanker,
        file_name: "mmarco-reranker/model.onnx",
        size_bytes: 135_702_676,
        files: &[
            ModelFile {
                file_name: "mmarco-reranker/model.onnx",
                url: "https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1/resolve/1427fd652930e4ba29e8149678df786c240d8825/onnx/model_quint8_avx2.onnx",
                size_bytes: 118_620_016,
                sha256: "6c2513767fb63d008a4377bef7a7a3555433d9436342bb53e35a3a72ffc52d4b",
            },
            ModelFile {
                file_name: "mmarco-reranker/tokenizer.json",
                url: "https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1/resolve/1427fd652930e4ba29e8149678df786c240d8825/tokenizer.json",
                size_bytes: 17_082_660,
                sha256: "62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626",
            },
        ],
        availability: ModelAvailability::Available,
        description: "Default multilingual cross-encoder for two-stage retrieval (Apache-2.0, CPU)",
        fallback_description: "Reciprocal Rank Fusion (RRF) ordering ($0.00 / Zero Network Leaks)",
    },
    OnnxModelSpec {
        id: "bge-reranker-v2-m3",
        name: "BGE-Reranker-v2-M3 Multilingual Cross-Encoder (INT8 ONNX)",
        category: OnnxModelCategory::ReRanker,
        file_name: "bge-reranker-v2-m3/model.onnx",
        size_bytes: 587_809_994,
        files: &[
            ModelFile {
                file_name: "bge-reranker-v2-m3/model.onnx",
                url: "https://huggingface.co/onnx-community/bge-reranker-v2-m3-ONNX/resolve/6f5ff65298512715a1e669753bc754d2bc8f367b/onnx/model_int8.onnx",
                size_bytes: 570_727_094,
                sha256: "912fc1215c2dbff6499700534bd8d31253af01573861abbfc43afd1fab6cce5d",
            },
            ModelFile {
                file_name: "bge-reranker-v2-m3/tokenizer.json",
                url: "https://huggingface.co/onnx-community/bge-reranker-v2-m3-ONNX/resolve/6f5ff65298512715a1e669753bc754d2bc8f367b/tokenizer.json",
                size_bytes: 17_082_900,
                sha256: "8bf8afbfd11306bd872018c53bfdf2e160a56f8edbcf49933324404791c148d3",
            },
        ],
        availability: ModelAvailability::Available,
        description: "Higher-quality multilingual cross-encoder for 32GB+ workstations (Apache-2.0, CPU)",
        fallback_description: "Reciprocal Rank Fusion (RRF) ordering ($0.00 / Zero Network Leaks)",
    },
    // --- Local multilingual embedding models (FEAT-067, ADR-0154). INT8 ONNX exports of the
    // MIT-licensed originals; the default of each profile is set by the retrieval benchmark.
    OnnxModelSpec {
        id: "multilingual-e5-small",
        name: "Multilingual E5 Small (INT8 ONNX, 384 dimensions)",
        category: OnnxModelCategory::Embedding,
        file_name: "multilingual-e5-small/model.onnx",
        size_bytes: 135_137_323,
        files: &[
            ModelFile {
                file_name: "multilingual-e5-small/model.onnx",
                url: "https://huggingface.co/Xenova/multilingual-e5-small/resolve/761b726dd34fb83930e26aab4e9ac3899aa1fa78/onnx/model_int8.onnx",
                size_bytes: 118_054_593,
                sha256: "4d24e2bc01a447951524466ef533e52944bf48509e6552810bcee1a2711cb02c",
            },
            ModelFile {
                file_name: "multilingual-e5-small/tokenizer.json",
                url: "https://huggingface.co/Xenova/multilingual-e5-small/resolve/761b726dd34fb83930e26aab4e9ac3899aa1fa78/tokenizer.json",
                size_bytes: 17_082_730,
                sha256: "0b44a9d7b51c3c62626640cda0e2c2f70fdacdc25bbbd68038369d14ebdf4c39",
            },
        ],
        availability: ModelAvailability::Available,
        description: "Local multilingual embeddings, 100+ languages (MIT, CPU)",
        fallback_description: "None: semantic search needs an embedding model",
    },
    OnnxModelSpec {
        id: "multilingual-e5-base",
        name: "Multilingual E5 Base (INT8 ONNX, 768 dimensions)",
        category: OnnxModelCategory::Embedding,
        file_name: "multilingual-e5-base/model.onnx",
        size_bytes: 295_266_822,
        files: &[
            ModelFile {
                file_name: "multilingual-e5-base/model.onnx",
                url: "https://huggingface.co/Xenova/multilingual-e5-base/resolve/1ec9243030a27d1a115d5c340572074c125b58b2/onnx/model_int8.onnx",
                size_bytes: 278_184_162,
                sha256: "9ddfd8b45086dabc59a7e1bb00463225dace8954962418b240840f2153bc87da",
            },
            ModelFile {
                file_name: "multilingual-e5-base/tokenizer.json",
                url: "https://huggingface.co/Xenova/multilingual-e5-base/resolve/1ec9243030a27d1a115d5c340572074c125b58b2/tokenizer.json",
                size_bytes: 17_082_660,
                sha256: "62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626",
            },
        ],
        availability: ModelAvailability::Available,
        description: "Local multilingual embeddings, 100+ languages (MIT, CPU)",
        fallback_description: "None: semantic search needs an embedding model",
    },
    OnnxModelSpec {
        id: "bge-m3",
        name: "BGE-M3 dense (INT8 ONNX, 1024 dimensions)",
        category: OnnxModelCategory::Embedding,
        file_name: "bge-m3/model.onnx",
        size_bytes: 585_539_515,
        files: &[
            ModelFile {
                file_name: "bge-m3/model.onnx",
                url: "https://huggingface.co/Xenova/bge-m3/resolve/4de13258303883538bd53b696b452bf8099f0858/onnx/model_int8.onnx",
                size_bytes: 568_456_694,
                sha256: "a206e10e995aa2a833924bcd725ba5dd6c3425cd34bac3cf2b5677cd2a1c51d6",
            },
            ModelFile {
                file_name: "bge-m3/tokenizer.json",
                url: "https://huggingface.co/Xenova/bge-m3/resolve/4de13258303883538bd53b696b452bf8099f0858/tokenizer.json",
                size_bytes: 17_082_821,
                sha256: "6710678b12670bc442b99edc952c4d996ae309a7020c1fa0096dd245c2faf790",
            },
        ],
        availability: ModelAvailability::Available,
        description: "Local multilingual embeddings, 100+ languages, long inputs (MIT, CPU)",
        fallback_description: "None: semantic search needs an embedding model",
    },
];

static SELECTED_RERANKER: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

/// Short label used in compact UI badges.
pub fn reranker_short_label(spec: &OnnxModelSpec) -> &'static str {
    match spec.id {
        "mmarco-reranker" => "mmarco-L12",
        "bge-reranker-v2-m3" => "bge-m3",
        other => other,
    }
}

/// Converts a `/`-separated catalog path into a platform path under `base`.
fn join_relative(base: PathBuf, rel: &str) -> PathBuf {
    rel.split('/').filter(|s| !s.is_empty()).fold(base, |acc, part| acc.join(part))
}

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
    /// True for catalog entries without a downloadable artifact yet.
    #[serde(default)]
    pub planned: bool,
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

        // Re-ranker aliases first: model names such as "mMiniLMv2-L12" contain digits that would
        // otherwise be captured by the Gemma tier aliases below.
        if clean == "reranker" || clean == "rerank" || clean == "mmarco" || clean.contains("mmarco") {
            return Self::find_spec(DEFAULT_RERANKER_ID);
        }
        if clean == "reranker-m3" || clean == "m3" || clean == "bge-m3" || clean == "bge-reranker" || clean.contains("reranker-v2") {
            return Self::find_spec("bge-reranker-v2-m3");
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
        if !spec.is_available() {
            return Err(Self::planned_message(spec));
        }
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

    /// User-facing message for catalog entries that have no downloadable artifact yet.
    pub fn planned_message(spec: &OnnxModelSpec) -> String {
        format!(
            "'{}' is planned and not available in this build (no download). AnyContext keeps using: {}.",
            spec.name, spec.fallback_description
        )
    }

    /// Resolves the absolute on-disk path of a model's primary file.
    pub fn model_path(spec: &OnnxModelSpec) -> PathBuf {
        join_relative(get_default_models_dir(), spec.file_name)
    }

    /// Resolves the absolute on-disk path of one of a model's files.
    pub fn file_path(file: &ModelFile) -> PathBuf {
        join_relative(get_default_models_dir(), file.file_name)
    }

    fn file_matches(file: &ModelFile) -> bool {
        std::fs::metadata(Self::file_path(file))
            .map(|m| m.is_file() && (file.size_bytes == 0 || m.len() == file.size_bytes))
            .unwrap_or(false)
    }

    /// True when every file of an available model is present with its exact expected size.
    pub fn is_installed(spec: &OnnxModelSpec) -> bool {
        spec.is_available() && !spec.files.is_empty() && spec.files.iter().all(Self::file_matches)
    }

    /// Returns the total on-disk size if at least one file is present.
    pub fn installed_size(spec: &OnnxModelSpec) -> Option<u64> {
        let sizes: Vec<u64> = spec
            .files
            .iter()
            .filter_map(|f| std::fs::metadata(Self::file_path(f)).ok().filter(|m| m.is_file()).map(|m| m.len()))
            .collect();
        if sizes.is_empty() { None } else { Some(sizes.iter().sum()) }
    }

    /// Inspects the store and produces an aggregated summary.
    pub fn inspect_store() -> ModelStoreSummary {
        let models_dir = get_default_models_dir();
        let mut installed_count = 0;
        let mut total_bytes = 0u64;
        let mut items = Vec::with_capacity(ONNX_CATALOG.len());

        for spec in ONNX_CATALOG {
            let installed = Self::is_installed(spec);
            let size = if installed { Self::installed_size(spec).unwrap_or(0) } else { 0 };
            if installed {
                installed_count += 1;
                total_bytes += size;
            }
            items.push(ModelStatusItem {
                id: spec.id.to_string(),
                name: spec.name.to_string(),
                category: spec.category.as_str().to_string(),
                is_installed: installed,
                file_path: Self::model_path(spec),
                disk_size_bytes: size,
                expected_size_bytes: spec.size_bytes,
                fallback_description: spec.fallback_description.to_string(),
                planned: !spec.is_available(),
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

    /// Document classifier state (Name, Detail, Is_Neural). Always heuristic in this build.
    pub fn active_ingestion_classifier() -> (&'static str, &'static str, bool) {
        let detail = match Self::find_spec("laya-int8") {
            Some(spec) if Self::is_installed(spec) => "Heuristic active (Laya weights installed; neural classification planned)",
            _ => "Heuristic active (Zero downloads / Sub-microsecond latency)",
        };
        ("Deterministic / Heuristic (<1µs / 0MB RAM)", detail, false)
    }

    /// Scanned-page classifier state. Always heuristic in this build.
    pub fn active_scanned_classifier() -> (&'static str, &'static str, bool) {
        let detail = match Self::find_spec("mobilenetv4-rvl-cdip") {
            Some(spec) if Self::is_installed(spec) => "Heuristic active (MobileNetV4 weights installed; neural sentinel planned)",
            _ => "Heuristic active (Pure Rust / Zero downloads)",
        };
        ("2D Spatial Heuristic (<5MB RAM)", detail, false)
    }

    /// Local vision state. Always heuristic in this build.
    pub fn active_vision_state() -> (&'static str, &'static str, bool) {
        let detail = match Self::find_spec("clip-vit-int8") {
            Some(spec) if Self::is_installed(spec) => "Heuristic active (CLIP weights installed; neural encoder planned)",
            _ => "Heuristic active (Zero downloads / Instant)",
        };
        ("2D & Typographic Heuristic (<5MB RAM)", detail, false)
    }

    /// Records the selected re-ranker (persisted in settings and cached for this process).
    pub fn set_active_reranker(id: &str) {
        if let Ok(db) = crate::storage::NativeConfigDb::open_default() {
            let _ = db.set_setting("reranker_model", id);
        }
        if let Ok(mut guard) = SELECTED_RERANKER.write() {
            *guard = Some(id.to_string());
        }
    }

    /// Re-ranker in effect: `ACTX_RERANKER_MODEL` (per-process override, e.g. `--rerank-model`),
    /// then the `reranker_model` setting (read once and cached), defaulting to `mmarco-reranker`.
    /// Cheap enough to call from UI render loops.
    pub fn active_reranker_spec() -> &'static OnnxModelSpec {
        let selected = std::env::var("ACTX_RERANKER_MODEL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                if let Some(cached) = SELECTED_RERANKER.read().ok().and_then(|g| g.clone()) {
                    return Some(cached);
                }
                let from_db = crate::storage::NativeConfigDb::open_default()
                    .ok()
                    .and_then(|db| db.get_setting("reranker_model").ok().flatten())
                    .unwrap_or_else(|| DEFAULT_RERANKER_ID.to_string());
                if let Ok(mut guard) = SELECTED_RERANKER.write() {
                    *guard = Some(from_db.clone());
                }
                Some(from_db)
            });
        selected
            .as_deref()
            .and_then(Self::find_spec)
            .filter(|s| s.category == OnnxModelCategory::ReRanker && s.is_available())
            .or_else(|| Self::find_spec(DEFAULT_RERANKER_ID))
            .expect("default re-ranker must exist in the catalog")
    }

    /// Lists catalog entries that are selectable re-rankers.
    pub fn rerankers() -> impl Iterator<Item = &'static OnnxModelSpec> {
        ONNX_CATALOG
            .iter()
            .filter(|s| s.category == OnnxModelCategory::ReRanker && s.is_available())
    }

    /// Neural re-ranker state (Name, Detail, Is_Neural). Neural only when the selected model's
    /// files are installed, the bundled ONNX Runtime library is present AND no load attempt of
    /// those weights has failed in this process.
    pub fn active_reranker_state() -> (&'static str, &'static str, bool) {
        let spec = Self::active_reranker_spec();
        if Self::is_installed(spec) {
            if actx_onnx::reranker::recorded_load_error(&Self::model_path(spec)).is_some() {
                return (
                    "Reciprocal Rank Fusion (RRF)",
                    "Fallback: the re-ranker failed to load (corrupt weights or incompatible ONNX Runtime; run `actx download reranker` to repair)",
                    false,
                );
            }
            if actx_onnx::runtime::runtime_library_present() {
                return (spec.name, "Neural cross-encoder active (ONNX Runtime, CPU)", true);
            }
            return (
                "Reciprocal Rank Fusion (RRF)",
                "Fallback: ONNX Runtime library not found next to the executable (reinstall or update AnyContext)",
                false,
            );
        }
        (
            "Reciprocal Rank Fusion (RRF)",
            "Fallback: re-ranker weights not installed (run `actx download reranker` or reinstall your profile)",
            false,
        )
    }

    /// Where model files are downloaded from: the mirror base URL set by `ACTX_MODELS_MIRROR`,
    /// else the `models_mirror` setting, else `None` (each file's pinned upstream URL). With a
    /// mirror, the public upstream is never contacted (FR-018).
    pub fn models_mirror() -> Option<String> {
        std::env::var(MODELS_MIRROR_ENV)
            .ok()
            .or_else(|| {
                crate::storage::NativeConfigDb::open_default()
                    .ok()
                    .and_then(|db| db.get_setting(MODELS_MIRROR_SETTING).ok().flatten())
            })
            .map(|m| m.trim().trim_end_matches('/').to_string())
            .filter(|m| !m.is_empty())
    }

    /// Download URL of `file` for the given mirror: `<mirror>/<file_name>`, or the pinned
    /// upstream URL without a mirror. The checksum is the catalog's either way.
    pub fn source_url(file: &ModelFile, mirror: Option<&str>) -> String {
        match mirror {
            Some(base) => format!("{}/{}", base.trim_end_matches('/'), file.file_name),
            None => file.url.to_string(),
        }
    }

    /// Checks every file of an installed model against its catalog SHA-256, so files placed by
    /// hand (or by IT) are only loaded when they are the verified ones (FR-014). The result is
    /// cached per file, size and modification time for the life of the process.
    pub fn verify_installed(spec: &OnnxModelSpec) -> Result<(), String> {
        use sha2::{Digest, Sha256};
        if !Self::is_installed(spec) {
            return Err(format!("'{}' is not installed in {}", spec.id, get_default_models_dir().display()));
        }
        static VERIFIED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<(PathBuf, u64, std::time::SystemTime)>>> =
            std::sync::OnceLock::new();
        let verified = VERIFIED.get_or_init(Default::default);
        for file in spec.files {
            let path = Self::file_path(file);
            let meta = std::fs::metadata(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let stamp = (path.clone(), meta.len(), meta.modified().unwrap_or(std::time::UNIX_EPOCH));
            if verified.lock().map(|v| v.contains(&stamp)).unwrap_or(false) {
                continue;
            }
            let mut hasher = Sha256::new();
            let mut reader = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            std::io::copy(&mut reader, &mut hasher).map_err(|e| format!("{}: {e}", path.display()))?;
            let digest = format!("{:x}", hasher.finalize());
            if !file.sha256.is_empty() && !digest.eq_ignore_ascii_case(file.sha256) {
                return Err(format!(
                    "{} does not match the verified file: SHA-256 {} instead of {}",
                    path.display(),
                    digest,
                    file.sha256
                ));
            }
            if let Ok(mut v) = verified.lock() {
                v.insert(stamp);
            }
        }
        Ok(())
    }

    /// Downloads every file of a model with streaming progress, SHA-256 verification and
    /// graceful cancellation. Files already present with the exact size are kept.
    /// Each file is written to a temporary path and atomically renamed only after its hash matches.
    pub async fn download_model_with_progress(
        spec: &OnnxModelSpec,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<(u64, u64)>>,
        cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<PathBuf, String> {
        if !spec.is_available() || spec.files.is_empty() {
            return Err(Self::planned_message(spec));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1800))
            .user_agent(concat!("AnyContext/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        let total_bytes = spec.size_bytes;
        let mut done_bytes: u64 = 0;
        let mirror = Self::models_mirror();

        for file in spec.files {
            if Self::file_matches(file) {
                done_bytes += file.size_bytes;
                if let Some(ref tx) = progress_tx {
                    let _ = tx.send((done_bytes, total_bytes));
                }
                continue;
            }
            let url = Self::source_url(file, mirror.as_deref());
            done_bytes += Self::download_file(&client, file, &url, done_bytes, total_bytes, &progress_tx, &cancel_rx).await?;
        }

        Ok(Self::model_path(spec))
    }

    async fn download_file(
        client: &reqwest::Client,
        file: &ModelFile,
        url: &str,
        offset: u64,
        total_bytes: u64,
        progress_tx: &Option<tokio::sync::mpsc::UnboundedSender<(u64, u64)>>,
        cancel_rx: &Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<u64, String> {
        use sha2::{Digest, Sha256};
        use std::io::Write;

        let dest_file = Self::file_path(file);
        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create models directory {:?}: {}", parent, e))?;
        }
        let tmp_file = dest_file.with_extension(format!(
            "{}.download.tmp",
            dest_file.extension().and_then(|e| e.to_str()).unwrap_or("bin")
        ));

        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to model source {}: {}", url, e))?;
        if !response.status().is_success() {
            return Err(format!("Model repository returned HTTP status {}: {}", response.status(), url));
        }

        let mut tmp_guard = PartialDownloadGuard::new(&tmp_file);
        let mut out = std::fs::File::create(&tmp_file)
            .map_err(|e| format!("Failed to create temporary model file {:?}: {}", tmp_file, e))?;
        let mut hasher = Sha256::new();
        let mut written: u64 = 0;
        let cancelled = |rx: &Option<tokio::sync::watch::Receiver<bool>>| rx.as_ref().map(|r| *r.borrow()).unwrap_or(false);

        while let Some(chunk) = response.chunk().await.map_err(|e| format!("Download stream error: {}", e))? {
            if cancelled(cancel_rx) {
                drop(out);
                let _ = std::fs::remove_file(&tmp_file);
                return Err("Download cancelled by user.".to_string());
            }
            out.write_all(&chunk).map_err(|e| format!("Failed to write chunk: {}", e))?;
            hasher.update(&chunk);
            written += chunk.len() as u64;
            if let Some(ref tx) = progress_tx {
                let _ = tx.send((offset + written, total_bytes));
            }
        }
        out.flush().map_err(|e| format!("Failed to flush file: {}", e))?;
        drop(out);

        if cancelled(cancel_rx) {
            let _ = std::fs::remove_file(&tmp_file);
            return Err("Download cancelled by user.".to_string());
        }

        let digest = format!("{:x}", hasher.finalize());
        if file.size_bytes > 0 && written != file.size_bytes {
            let _ = std::fs::remove_file(&tmp_file);
            return Err(format!(
                "Size mismatch for {}: expected {} bytes, received {} bytes. The file was discarded.",
                file.file_name, file.size_bytes, written
            ));
        }
        if !file.sha256.is_empty() && !digest.eq_ignore_ascii_case(file.sha256) {
            let _ = std::fs::remove_file(&tmp_file);
            return Err(format!(
                "Integrity check failed for {}: SHA-256 {} does not match the expected {}. The file was discarded.",
                file.file_name, digest, file.sha256
            ));
        }

        if dest_file.exists() {
            let _ = std::fs::remove_file(&dest_file);
        }
        std::fs::rename(&tmp_file, &dest_file)
            .map_err(|e| format!("Failed to finalize model file {:?}: {}", dest_file, e))?;
        tmp_guard.disarm();
        Ok(written)
    }

    /// Asynchronously downloads a model specification with standard streaming.
    pub async fn download_model(spec: &OnnxModelSpec) -> Result<PathBuf, String> {
        Self::download_model_with_progress(spec, None, None).await
    }
}

/// Removes a partially written download on every early return; disarmed once the file is final.
struct PartialDownloadGuard {
    path: PathBuf,
    armed: bool,
}

impl PartialDownloadGuard {
    fn new(path: &Path) -> Self {
        Self { path: path.to_path_buf(), armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PartialDownloadGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_integrity() {
        let catalog = OnnxModelManager::catalog();
        assert_eq!(catalog.len(), 14);
        for id in [
            "gemma-4-compact", "gemma-4-12b", "gemma-4-27b", "gemma-4-32b", "laya-int8",
            "mobilenetv4-rvl-cdip", "layoutlmv3-int8", "clip-vit-int8", "bge-small-onnx",
            "mmarco-reranker", "bge-reranker-v2-m3", "multilingual-e5-small", "multilingual-e5-base", "bge-m3",
        ] {
            assert!(catalog.iter().any(|m| m.id == id), "missing {id}");
        }
        assert!(!catalog.iter().any(|m| m.id == "bge-reranker-small"));

        for spec in catalog {
            assert!(!spec.name.is_empty() && !spec.fallback_description.is_empty());
            match spec.availability {
                ModelAvailability::Planned => {
                    assert!(spec.files.is_empty(), "{} is planned but lists files", spec.id);
                    assert_eq!(spec.size_bytes, 0);
                }
                ModelAvailability::Available => {
                    assert!(!spec.files.is_empty(), "{} has no files", spec.id);
                    assert_eq!(spec.files.iter().map(|f| f.size_bytes).sum::<u64>(), spec.size_bytes, "{} size total", spec.id);
                    assert!(spec.files.iter().any(|f| f.file_name == spec.file_name), "{} primary file", spec.id);
                    for f in spec.files {
                        assert!(f.size_bytes > 0);
                        assert_eq!(f.sha256.len(), 64, "{} sha256", f.file_name);
                        assert!(f.sha256.chars().all(|c| c.is_ascii_hexdigit()));
                        assert!(f.url.starts_with("https://huggingface.co/"), "{}", f.url);
                        assert!(!f.url.contains("/resolve/main/"), "{} must pin a revision", f.url);
                    }
                }
            }
        }
        assert!(catalog.iter().filter(|m| m.category == OnnxModelCategory::LanguageModel).all(|m| !m.is_available()));
    }

    #[test]
    fn test_rerankers_ship_tokenizer() {
        for spec in OnnxModelManager::rerankers() {
            assert!(spec.files.iter().any(|f| f.file_name.ends_with("/tokenizer.json")), "{}", spec.id);
            assert!(spec.file_name.ends_with("/model.onnx"));
        }
    }

    #[test]
    fn test_inspect_store_does_not_panic() {
        let summary = OnnxModelManager::inspect_store();
        assert_eq!(summary.total_count, 14);
        assert_eq!(summary.models.len(), 14);
    }

    #[test]
    fn test_planned_models_never_install_or_download() {
        let spec = OnnxModelManager::find_spec("gemma-4-compact").unwrap();
        assert!(!OnnxModelManager::is_installed(spec));
        let err = OnnxModelManager::check_download_safety(spec, true).unwrap_err();
        assert!(err.contains("planned"));
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let res = rt.block_on(OnnxModelManager::download_model(spec));
        assert!(res.unwrap_err().contains("planned"));
    }

    #[test]
    fn test_resolve_spec_aliases() {
        assert_eq!(OnnxModelManager::resolve_spec("compact").unwrap().id, "gemma-4-compact");
        assert_eq!(OnnxModelManager::resolve_spec("12b").unwrap().id, "gemma-4-12b");
        assert_eq!(OnnxModelManager::resolve_spec("gemma-4-27b").unwrap().id, "gemma-4-27b");
        assert_eq!(OnnxModelManager::resolve_spec("32b").unwrap().id, "gemma-4-32b");
        assert_eq!(OnnxModelManager::resolve_spec("laya").unwrap().id, "laya-int8");
        assert_eq!(OnnxModelManager::resolve_spec("scans").unwrap().id, "mobilenetv4-rvl-cdip");
        assert_eq!(OnnxModelManager::resolve_spec("reranker").unwrap().id, "mmarco-reranker");
        assert_eq!(OnnxModelManager::resolve_spec("mmarco-mMiniLMv2-L12").unwrap().id, "mmarco-reranker");
        assert_eq!(OnnxModelManager::resolve_spec("reranker-m3").unwrap().id, "bge-reranker-v2-m3");
        assert_eq!(OnnxModelManager::resolve_spec("unknown-nonexistent"), None);
    }

    #[test]
    fn test_join_relative_uses_platform_separators() {
        let p = join_relative(PathBuf::from("base"), "mmarco-reranker/model.onnx");
        assert_eq!(p, PathBuf::from("base").join("mmarco-reranker").join("model.onnx"));
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

    /// Live verification of every downloadable catalog file (ADR-118 / ADR-130).
    /// Run explicitly in the release workflow: `cargo test -p any-context-core-rs catalog_urls -- --ignored`.
    #[test]
    #[ignore]
    fn catalog_urls() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let client = reqwest::Client::builder()
                .user_agent(concat!("AnyContext/", env!("CARGO_PKG_VERSION")))
                .redirect(reqwest::redirect::Policy::limited(10))
                .build()
                .unwrap();
            let mut failures = Vec::new();
            for spec in OnnxModelManager::catalog().iter().filter(|s| s.is_available()) {
                for f in spec.files {
                    match client.head(f.url).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            let len = resp.content_length().unwrap_or(0);
                            if len != f.size_bytes {
                                failures.push(format!("{}: size {} != {}", f.url, len, f.size_bytes));
                            }
                        }
                        Ok(resp) => failures.push(format!("{}: HTTP {}", f.url, resp.status())),
                        Err(e) => failures.push(format!("{}: {}", f.url, e)),
                    }
                }
            }
            assert!(failures.is_empty(), "Broken catalog entries:\n{}", failures.join("\n"));
        });
    }
}
