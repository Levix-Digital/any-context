//! # 🧠 `actx-onnx`: Local ONNX Inference Crate
//!
//! Component status in this build (ADR-130):
//! - Text embedding models (multilingual-e5, BGE-M3): **neural**, ONNX Runtime on CPU
//!   (requires the `runtime` feature).
//! - Cross-Encoder Re-Ranker (mmarco-mMiniLMv2 / bge-reranker-v2-m3): **neural**, ONNX Runtime on CPU
//!   (requires the `runtime` feature).
//! - Query Classifier, Scan Sentinel, Layout Extractor, Vision Encoder: **heuristic**
//!   (deterministic Rust; downloaded weights are not executed yet).
//! - Gemma 4 local generation: **planned** (no generative runtime yet).

pub mod runtime;
pub mod tokenizer;
pub mod session;
pub mod query_classifier;
pub mod scan_sentinel;
pub mod layout_extractor;
pub mod vision_encoder;
pub mod gemma;
pub mod engine;
pub mod device;
pub mod reranker;
pub mod embedder;

pub use session::ModelSession;
pub use query_classifier::{OnnxComplexity, OnnxIntent, OnnxQueryClassifier};
pub use scan_sentinel::OnnxScanSentinel;
pub use layout_extractor::OnnxLayoutExtractor;
pub use vision_encoder::OnnxVisionEncoder;
pub use gemma::{GemmaModelTier, GemmaTokenizer, OnnxGemmaEngine};
pub use engine::{ComponentStatus, OnnxEngine, DEFAULT_RERANKER_DIR};
pub use device::{AcceleratorBackend, AcceleratorVendor, ExecutionDevice, GpuDevice};
pub use reranker::OnnxReranker;
pub use embedder::{EmbedderConfig, EmbeddingPooling, OnnxEmbedder};
pub use runtime::{runtime_available, OnnxSession, TensorOutput};
