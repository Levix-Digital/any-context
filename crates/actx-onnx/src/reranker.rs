//! ONNX Cross-Encoder Neural Re-Ranker (ADR-130).
//!
//! Scores `(query, document)` pairs with a real cross-encoder executed by ONNX Runtime
//! (CPU Execution Provider). The default model is `mmarco-mMiniLMv2-L12-H384-v1`
//! (multilingual, XLM-RoBERTa); `bge-reranker-v2-m3` uses the same code path.
//!
//! Each model lives in its own directory containing `model.onnx` and `tokenizer.json`.
//! The ONNX session is loaded lazily on the first scoring call, never at startup.
//!
//! Architectural Invariant: ADR-110 Zero-Breakage Guarantee.
//! Missing, corrupt or unloadable weights make `score_batch` return an empty vector,
//! signalling the pipeline to keep the Stage-1 RRF ordering.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::device::ExecutionDevice;
use crate::runtime::{self, OnnxSession, TensorOutput};
use crate::tokenizer::{self, PairTokenizer};

/// Maximum tokens per `(query, document)` pair.
pub const DEFAULT_MAX_LENGTH: usize = 512;
/// Pairs scored per ONNX Runtime call.
pub const BATCH_SIZE: usize = 16;
/// Tokenizer file expected next to `model.onnx`.
pub const TOKENIZER_FILE_NAME: &str = "tokenizer.json";

struct LoadedReranker {
    session: OnnxSession,
    tokenizer: PairTokenizer,
}

/// Cross-encoder re-ranker backed by ONNX weights on disk.
#[derive(Clone)]
pub struct OnnxReranker {
    model_path: PathBuf,
    tokenizer_path: PathBuf,
    model_id: String,
    max_length: usize,
    device: ExecutionDevice,
    state: Arc<OnceLock<Result<LoadedReranker, String>>>,
}

impl std::fmt::Debug for OnnxReranker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxReranker")
            .field("model_id", &self.model_id)
            .field("model_path", &self.model_path)
            .field("device", &self.device)
            .field("loaded", &self.state.get().map(|r| r.is_ok()))
            .finish()
    }
}

impl OnnxReranker {
    /// Creates a reranker for `model_path`, reading `tokenizer.json` from the same directory.
    pub fn new(model_path: &Path) -> Self {
        Self::with_device(model_path, ExecutionDevice::Auto)
    }

    /// Creates a reranker with an explicit requested execution device.
    pub fn with_device(model_path: &Path, device: ExecutionDevice) -> Self {
        let tokenizer_path = model_path
            .parent()
            .map(|p| p.join(TOKENIZER_FILE_NAME))
            .unwrap_or_else(|| PathBuf::from(TOKENIZER_FILE_NAME));
        let model_id = model_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("reranker")
            .to_string();
        Self::with_paths(model_path, &tokenizer_path, &model_id, device)
    }

    /// Creates a reranker from explicit model and tokenizer paths.
    pub fn with_paths(model_path: &Path, tokenizer_path: &Path, model_id: &str, device: ExecutionDevice) -> Self {
        Self {
            model_path: model_path.to_path_buf(),
            tokenizer_path: tokenizer_path.to_path_buf(),
            model_id: model_id.to_string(),
            max_length: DEFAULT_MAX_LENGTH,
            device,
            state: Arc::new(OnceLock::new()),
        }
    }

    /// Overrides the per-pair token budget (useful for small fixtures and benchmarks).
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length.max(8);
        self.state = Arc::new(OnceLock::new());
        self
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// True when both weight files exist on disk (does not load them).
    pub fn weights_present(&self) -> bool {
        let non_empty = |p: &Path| std::fs::metadata(p).map(|m| m.is_file() && m.len() > 0).unwrap_or(false);
        non_empty(&self.model_path) && non_empty(&self.tokenizer_path)
    }

    /// True when neural scoring is expected to run: ONNX Runtime is linked, weights are
    /// present and no load attempt has failed. A failed load flips this to `false`.
    pub fn is_neural_active(&self) -> bool {
        if !runtime::runtime_library_present() || !self.weights_present() {
            return false;
        }
        !matches!(self.state.get(), Some(Err(_)))
    }

    /// Error recorded by the last load attempt, if any.
    pub fn load_error(&self) -> Option<String> {
        match self.state.get() {
            Some(Err(e)) => Some(e.clone()),
            _ => None,
        }
    }

    /// Loads the ONNX session and tokenizer if not already loaded.
    pub fn ensure_loaded(&self) -> Result<(), String> {
        match self.loaded() {
            Ok(_) => Ok(()),
            Err(e) => Err(e.clone()),
        }
    }

    fn loaded(&self) -> Result<&LoadedReranker, &String> {
        self.state
            .get_or_init(|| {
                if !self.weights_present() {
                    return Err(format!(
                        "Reranker weights for '{}' not found (expected {} and {})",
                        self.model_id,
                        self.model_path.display(),
                        self.tokenizer_path.display()
                    ));
                }
                let session = runtime::load_session(&self.model_path, &self.model_id)?;
                if !session.has_input("input_ids") || !session.has_input("attention_mask") {
                    return Err(format!(
                        "Model '{}' is not a supported cross-encoder (inputs: {:?})",
                        self.model_id,
                        session.input_names()
                    ));
                }
                let tokenizer = tokenizer::load_pair_tokenizer(&self.tokenizer_path, self.max_length)?;
                Ok(LoadedReranker { session, tokenizer })
            })
            .as_ref()
    }

    /// Records the requested device. Inference always runs on `effective_device()`.
    pub fn set_device(&mut self, device: ExecutionDevice) {
        self.device = device;
    }

    /// Requested execution device (as configured by the user).
    pub fn device(&self) -> ExecutionDevice {
        self.device
    }

    /// Device where inference actually executes in this build (CPU only, ADR-130).
    pub fn effective_device(&self) -> ExecutionDevice {
        self.device.effective()
    }

    /// Scores candidates against the query. Returns scores in `[0.0, 1.0]`, one per
    /// candidate, or an empty vector when neural scoring is unavailable (RRF fallback).
    pub fn score_batch(&self, query: &str, candidates: &[String]) -> Vec<f32> {
        if candidates.is_empty() || !self.is_neural_active() {
            return Vec::new();
        }
        self.try_score_batch(query, candidates).unwrap_or_default()
    }

    /// Like `score_batch`, but surfaces the failure reason.
    pub fn try_score_batch(&self, query: &str, candidates: &[String]) -> Result<Vec<f32>, String> {
        let loaded = self.loaded().map_err(|e| e.clone())?;
        let query = query.trim();
        let mut scores = Vec::with_capacity(candidates.len());

        for chunk in candidates.chunks(BATCH_SIZE) {
            let batch = loaded.tokenizer.encode_pairs(query, chunk)?;
            if batch.batch_size != chunk.len() || batch.seq_len == 0 {
                return Err(format!("Tokenizer produced an invalid batch for '{}'", self.model_id));
            }
            let mut inputs = vec![
                ("input_ids".to_string(), batch.input_ids),
                ("attention_mask".to_string(), batch.attention_mask),
            ];
            if loaded.session.has_input("token_type_ids") {
                inputs.push(("token_type_ids".to_string(), batch.token_type_ids));
            }
            let output = loaded.session.run_i64([batch.batch_size, batch.seq_len], inputs)?;
            scores.extend(Self::logits_to_scores(&output, chunk.len())?);
        }

        Ok(scores)
    }

    /// Scores a single pair; returns `0.0` when neural scoring is unavailable.
    pub fn score_pair(&self, query: &str, document: &str) -> f32 {
        self.score_batch(query, &[document.to_string()]).first().copied().unwrap_or(0.0)
    }

    /// Converts raw model outputs into calibrated relevance probabilities.
    /// `[batch]` / `[batch, 1]` logits use a sigmoid; `[batch, 2]` uses softmax (positive class).
    pub fn logits_to_scores(output: &TensorOutput, batch: usize) -> Result<Vec<f32>, String> {
        if batch == 0 {
            return Ok(Vec::new());
        }
        if output.data.len() == batch {
            Ok(output.data.iter().map(|&x| Self::sigmoid(x)).collect())
        } else if output.data.len() == batch * 2 {
            Ok(output
                .data
                .chunks(2)
                .map(|row| {
                    let m = row[0].max(row[1]);
                    let (e0, e1) = ((row[0] - m).exp(), (row[1] - m).exp());
                    e1 / (e0 + e1)
                })
                .collect())
        } else {
            Err(format!(
                "Unexpected reranker output shape {:?} for batch of {}",
                output.shape, batch
            ))
        }
    }

    /// Standard sigmoid mapping real logits to `[0.0, 1.0]`.
    #[inline]
    pub fn sigmoid(x: f32) -> f32 {
        1.0 / (1.0 + (-x).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reranker_missing_file_fallback() {
        let reranker = OnnxReranker::new(Path::new("missing_dir/model.onnx"));
        assert!(!reranker.is_neural_active());
        let scores = reranker.score_batch("test query", &["doc 1".to_string(), "doc 2".to_string()]);
        assert!(scores.is_empty(), "Should return empty vector to signal fallback to RRF");
    }

    #[test]
    fn test_reranker_corrupt_weights_never_active_after_load() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("model.onnx");
        let mut bytes = vec![0x08, 0x07, 0x12, 0x04];
        bytes.resize(2048, 0xAB);
        std::fs::write(&model, &bytes).unwrap();
        std::fs::write(dir.path().join(TOKENIZER_FILE_NAME), "{}").unwrap();

        let reranker = OnnxReranker::new(&model);
        assert!(reranker.score_batch("q", &["d".to_string()]).is_empty());
        assert!(reranker.ensure_loaded().is_err());
        assert!(!reranker.is_neural_active(), "Dummy bytes must never be reported as neural");
        assert!(reranker.load_error().is_some());
    }

    #[test]
    fn test_reranker_sigmoid_bounds() {
        assert!((OnnxReranker::sigmoid(0.0) - 0.5).abs() < 1e-4);
        assert!(OnnxReranker::sigmoid(100.0) > 0.999);
        assert!(OnnxReranker::sigmoid(-100.0) < 0.001);
    }

    #[test]
    fn test_logits_to_scores_shapes() {
        let single = TensorOutput { shape: vec![2, 1], data: vec![0.0, 10.0] };
        let s = OnnxReranker::logits_to_scores(&single, 2).unwrap();
        assert!((s[0] - 0.5).abs() < 1e-4 && s[1] > 0.99);

        let two_class = TensorOutput { shape: vec![1, 2], data: vec![0.0, 0.0] };
        let s = OnnxReranker::logits_to_scores(&two_class, 1).unwrap();
        assert!((s[0] - 0.5).abs() < 1e-4);

        let bad = TensorOutput { shape: vec![3], data: vec![0.0, 0.0, 0.0] };
        assert!(OnnxReranker::logits_to_scores(&bad, 2).is_err());
    }

    #[test]
    fn test_effective_device_is_cpu() {
        let r = OnnxReranker::with_device(Path::new("x/model.onnx"), ExecutionDevice::DirectML { adapter_index: 0 });
        assert_eq!(r.effective_device(), ExecutionDevice::Cpu);
    }

    #[cfg(feature = "runtime")]
    mod fixture {
        use super::*;

        /// Fixture tests need the ONNX Runtime library (`ORT_DYLIB_PATH`). They are skipped
        /// when it is absent, unless `ACTX_REQUIRE_ORT=1` (set in CI) makes it mandatory.
        fn runtime_ready() -> bool {
            if runtime::probe_runtime().is_ok() {
                return true;
            }
            if std::env::var("ACTX_REQUIRE_ORT").as_deref() == Ok("1") {
                panic!("ACTX_REQUIRE_ORT=1 but the ONNX Runtime library could not be loaded: {:?}", runtime::probe_runtime());
            }
            eprintln!("skipping: ONNX Runtime library unavailable (set ORT_DYLIB_PATH)");
            false
        }

        fn fixture_reranker() -> OnnxReranker {
            let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            OnnxReranker::with_paths(
                &dir.join("tiny_reranker.onnx"),
                &dir.join("tiny_tokenizer.json"),
                "tiny-fixture",
                ExecutionDevice::Cpu,
            )
            .with_max_length(32)
        }

        #[test]
        fn test_fixture_scores_in_unit_interval_and_ordered() {
            if !runtime_ready() {
                return;
            }
            let r = fixture_reranker();
            assert!(r.is_neural_active());
            let docs = vec![
                "the theme colors".to_string(),
                "timeout is thirty seconds".to_string(),
            ];
            let scores = r.try_score_batch("what timeout", &docs).expect("fixture inference runs");
            assert_eq!(scores.len(), 2);
            assert!(scores.iter().all(|s| (0.0..=1.0).contains(s)));
            assert!(scores[1] > scores[0], "timeout doc must outrank theme doc: {:?}", scores);
        }

        #[test]
        fn test_fixture_deterministic_and_batch_invariant() {
            if !runtime_ready() {
                return;
            }
            let r = fixture_reranker();
            let docs: Vec<String> = (0..(BATCH_SIZE + 3))
                .map(|i| if i % 2 == 0 { "timeout seconds".to_string() } else { "theme".to_string() })
                .collect();
            let batched = r.try_score_batch("what timeout", &docs).unwrap();
            let again = r.try_score_batch("what timeout", &docs).unwrap();
            assert_eq!(batched, again);
            for (i, d) in docs.iter().enumerate() {
                let single = r.try_score_batch("what timeout", &[d.clone()]).unwrap()[0];
                assert!((single - batched[i]).abs() < 1e-5, "batch/single mismatch at {i}");
            }
        }
    }
}
