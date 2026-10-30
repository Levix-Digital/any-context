//! ONNX text embedding model (FEAT-067, ADR-0154).
//!
//! Turns texts into one normalized vector each with a local transformer encoder executed by ONNX
//! Runtime: tokenize, run the graph, pool the token states (mean under the attention mask, or the
//! first token) and L2-normalize. The default models are multilingual-e5 and BGE-M3 INT8 exports.
//!
//! Each model lives in its own directory containing `model.onnx` and `tokenizer.json`. The
//! session is loaded lazily on the first call. Unlike the reranker, a failure is an error for the
//! caller: embeddings never fall back to anything (IMP-028).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use crate::runtime::{self, OnnxSession, TensorOutput};
use crate::tokenizer::{self, PairTokenizer};

/// Texts embedded per ONNX Runtime call.
pub const EMBED_BATCH_SIZE: usize = 16;

/// How token states become one vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingPooling {
    /// Mean of the token states under the attention mask (e5).
    Mean,
    /// State of the first token (BGE-M3).
    Cls,
}

/// What an embedding model needs to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbedderConfig {
    pub model_id: String,
    pub model_path: PathBuf,
    pub tokenizer_path: PathBuf,
    pub dimension: usize,
    pub max_tokens: usize,
    pub pooling: EmbeddingPooling,
}

struct LoadedEmbedder {
    session: OnnxSession,
    tokenizer: PairTokenizer,
}

/// A local embedding model backed by ONNX weights on disk.
#[derive(Clone)]
pub struct OnnxEmbedder {
    config: EmbedderConfig,
    state: Arc<OnceLock<Result<LoadedEmbedder, String>>>,
    truncated: Arc<AtomicUsize>,
}

impl std::fmt::Debug for OnnxEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnnxEmbedder")
            .field("model_id", &self.config.model_id)
            .field("model_path", &self.config.model_path)
            .field("loaded", &self.state.get().map(|r| r.is_ok()))
            .finish()
    }
}

impl OnnxEmbedder {
    pub fn new(config: EmbedderConfig) -> Self {
        Self {
            config,
            state: Arc::new(OnceLock::new()),
            truncated: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn config(&self) -> &EmbedderConfig {
        &self.config
    }

    /// Texts cut at the model's input limit since this embedder was created.
    pub fn truncated_inputs(&self) -> usize {
        self.truncated.load(Ordering::Relaxed)
    }

    /// Loads the session and tokenizer if not already loaded.
    pub fn ensure_loaded(&self) -> Result<(), String> {
        self.loaded().map(|_| ()).map_err(|e| e.clone())
    }

    fn loaded(&self) -> Result<&LoadedEmbedder, &String> {
        self.state.get_or_init(|| self.load()).as_ref()
    }

    fn load(&self) -> Result<LoadedEmbedder, String> {
        let c = &self.config;
        for path in [&c.model_path, &c.tokenizer_path] {
            if !std::fs::metadata(path).map(|m| m.is_file() && m.len() > 0).unwrap_or(false) {
                return Err(format!("Embedding model '{}' file not found: {}", c.model_id, path.display()));
            }
        }
        let session = runtime::load_session(&c.model_path, &c.model_id)?;
        if !session.has_input("input_ids") || !session.has_input("attention_mask") {
            return Err(format!(
                "Model '{}' is not a supported text encoder (inputs: {:?})",
                c.model_id,
                session.input_names()
            ));
        }
        let tokenizer = tokenizer::load_pair_tokenizer(&c.tokenizer_path, c.max_tokens)?;
        Ok(LoadedEmbedder { session, tokenizer })
    }

    /// One normalized vector per text, in input order. Blocking: call it off the async runtime.
    pub fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        let loaded = self.loaded().map_err(|e| e.clone())?;
        let mut vectors = Vec::with_capacity(texts.len());
        for chunk in texts.chunks(EMBED_BATCH_SIZE) {
            let (batch, truncated) = loaded.tokenizer.encode_texts(chunk)?;
            if batch.batch_size != chunk.len() || batch.seq_len == 0 {
                return Err(format!("Tokenizer produced an invalid batch for '{}'", self.config.model_id));
            }
            self.truncated.fetch_add(truncated, Ordering::Relaxed);
            let mask = batch.attention_mask.clone();
            let mut inputs = vec![
                ("input_ids".to_string(), batch.input_ids),
                ("attention_mask".to_string(), batch.attention_mask),
            ];
            if loaded.session.has_input("token_type_ids") {
                inputs.push(("token_type_ids".to_string(), batch.token_type_ids));
            }
            let output = loaded.session.run_i64([batch.batch_size, batch.seq_len], inputs)?;
            vectors.extend(pool(&output, &mask, self.config.pooling, self.config.dimension)?);
        }
        Ok(vectors)
    }
}

/// Pools a `[batch, seq, dim]` output into one L2-normalized vector per row.
pub fn pool(output: &TensorOutput, mask: &[i64], pooling: EmbeddingPooling, dimension: usize) -> Result<Vec<Vec<f32>>, String> {
    let [batch, seq, dim] = match output.shape.as_slice() {
        [b, s, d] => [*b as usize, *s as usize, *d as usize],
        other => return Err(format!("Expected token states [batch, seq, dim], got shape {other:?}")),
    };
    if dim != dimension {
        return Err(format!("Model produced {dim} dimensions, the catalog says {dimension}"));
    }
    if output.data.len() != batch * seq * dim || mask.len() != batch * seq {
        return Err("Token states and attention mask sizes do not match".to_string());
    }
    let mut out = Vec::with_capacity(batch);
    for b in 0..batch {
        let rows = &output.data[b * seq * dim..(b + 1) * seq * dim];
        let mut v = vec![0.0f32; dim];
        match pooling {
            EmbeddingPooling::Cls => v.copy_from_slice(&rows[..dim]),
            EmbeddingPooling::Mean => {
                let mut count = 0.0f32;
                for t in 0..seq {
                    if mask[b * seq + t] == 0 {
                        continue;
                    }
                    count += 1.0;
                    for (acc, x) in v.iter_mut().zip(&rows[t * dim..(t + 1) * dim]) {
                        *acc += x;
                    }
                }
                if count > 0.0 {
                    v.iter_mut().for_each(|x| *x /= count);
                }
            }
        }
        normalize(&mut v);
        out.push(v);
    }
    Ok(out)
}

fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Paths of a model stored as `<dir>/model.onnx` and `<dir>/tokenizer.json`.
pub fn model_files(dir: &Path) -> (PathBuf, PathBuf) {
    (dir.join("model.onnx"), dir.join("tokenizer.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states(shape: [i64; 3], data: Vec<f32>) -> TensorOutput {
        TensorOutput { shape: shape.to_vec(), data }
    }

    #[test]
    fn mean_pooling_ignores_padding_and_normalizes() {
        // Two rows of two tokens, dimension 2; the second row's second token is padding.
        let out = states([2, 2, 2], vec![1.0, 0.0, 3.0, 0.0, 0.0, 2.0, 9.0, 9.0]);
        let v = pool(&out, &[1, 1, 1, 0], EmbeddingPooling::Mean, 2).unwrap();
        assert_eq!(v[0], vec![1.0, 0.0]);
        assert_eq!(v[1], vec![0.0, 1.0]);
    }

    #[test]
    fn cls_pooling_takes_the_first_token() {
        let out = states([1, 2, 2], vec![3.0, 4.0, 100.0, 100.0]);
        let v = pool(&out, &[1, 1], EmbeddingPooling::Cls, 2).unwrap();
        assert!((v[0][0] - 0.6).abs() < 1e-6 && (v[0][1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn wrong_shapes_are_errors() {
        let out = states([1, 2, 3], vec![0.0; 6]);
        assert!(pool(&out, &[1, 1], EmbeddingPooling::Mean, 2).is_err());
        let flat = TensorOutput { shape: vec![1, 2], data: vec![0.0; 2] };
        assert!(pool(&flat, &[1], EmbeddingPooling::Mean, 2).is_err());
    }

    #[test]
    fn missing_files_are_an_error_never_a_vector() {
        let (model_path, tokenizer_path) = model_files(Path::new("missing-model-dir"));
        let embedder = OnnxEmbedder::new(EmbedderConfig {
            model_id: "missing".into(),
            model_path,
            tokenizer_path,
            dimension: 4,
            max_tokens: 16,
            pooling: EmbeddingPooling::Mean,
        });
        assert!(embedder.embed(&["text".to_string()]).is_err());
        assert!(embedder.ensure_loaded().is_err());
    }
}
