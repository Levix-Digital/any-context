//! # 🧠 `actx-onnx`: Specialized Local ONNX Neural Inference Crate
//!
//! Governs the execution of standalone local ONNX models:
//! - BGE-Small Query Classifier (RFC-042 Dynamic Complexity)
//! - MobileNetV4 RVL-CDIP (Scanned Page Sentinel)
//! - LayoutLMv3 Base & Laya mmBERT (Multimodal Layout & Tables)
//! - CLIP ViT Vision Encoder (Dense Visual Diagrams)
//! - Gemma 4 Family Tensor Runner (Compact, 12B, 27B, 32B INT4)

pub mod session;
pub mod query_classifier;
pub mod scan_sentinel;
pub mod layout_extractor;
pub mod vision_encoder;
pub mod gemma;
pub mod engine;

pub use session::ModelSession;
pub use query_classifier::{OnnxComplexity, OnnxIntent, OnnxQueryClassifier};
pub use scan_sentinel::OnnxScanSentinel;
pub use layout_extractor::OnnxLayoutExtractor;
pub use vision_encoder::OnnxVisionEncoder;
pub use gemma::{GemmaModelTier, GemmaTokenizer, OnnxGemmaEngine};
pub use engine::OnnxEngine;
