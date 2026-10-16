//! Hugging Face `tokenizer.json` wrapper for cross-encoder `(query, document)` pairs (ADR-130).
//!
//! Produces padded `input_ids`, `attention_mask` and `token_type_ids` batches ready
//! for ONNX Runtime, truncating each pair with the `longest_first` strategy.

use std::path::Path;

/// A padded batch of encoded `(query, document)` pairs in row-major layout.
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedPairBatch {
    pub batch_size: usize,
    pub seq_len: usize,
    pub input_ids: Vec<i64>,
    pub attention_mask: Vec<i64>,
    pub token_type_ids: Vec<i64>,
}

#[cfg(feature = "runtime")]
mod imp {
    use super::EncodedPairBatch;
    use std::path::Path;
    use tokenizers::{PaddingDirection, PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

    /// Pair tokenizer loaded from a Hugging Face `tokenizer.json`.
    pub struct PairTokenizer {
        inner: Tokenizer,
        max_length: usize,
    }

    impl std::fmt::Debug for PairTokenizer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("PairTokenizer").field("max_length", &self.max_length).finish()
        }
    }

    impl PairTokenizer {
        pub fn from_file(path: &Path, max_length: usize) -> Result<Self, String> {
            let mut inner = Tokenizer::from_file(path)
                .map_err(|e| format!("Failed to load tokenizer from {}: {e}", path.display()))?;

            inner
                .with_truncation(Some(TruncationParams {
                    max_length,
                    strategy: TruncationStrategy::LongestFirst,
                    ..Default::default()
                }))
                .map_err(|e| format!("Invalid truncation settings for {}: {e}", path.display()))?;

            let padding = match inner.get_padding() {
                Some(p) => PaddingParams {
                    strategy: PaddingStrategy::BatchLongest,
                    direction: PaddingDirection::Right,
                    ..p.clone()
                },
                None => {
                    let (pad_token, pad_id) = ["<pad>", "[PAD]"]
                        .iter()
                        .find_map(|t| inner.token_to_id(t).map(|id| (t.to_string(), id)))
                        .unwrap_or_else(|| ("<pad>".to_string(), 0));
                    PaddingParams {
                        strategy: PaddingStrategy::BatchLongest,
                        direction: PaddingDirection::Right,
                        pad_id,
                        pad_token,
                        ..Default::default()
                    }
                }
            };
            inner.with_padding(Some(padding));

            Ok(Self { inner, max_length })
        }

        pub fn max_length(&self) -> usize {
            self.max_length
        }

        pub fn encode_pairs(&self, query: &str, documents: &[String]) -> Result<EncodedPairBatch, String> {
            let inputs: Vec<(String, String)> = documents
                .iter()
                .map(|d| (query.to_string(), d.clone()))
                .collect();
            let encodings = self
                .inner
                .encode_batch(inputs, true)
                .map_err(|e| format!("Tokenization failed: {e}"))?;

            let batch_size = encodings.len();
            let seq_len = encodings.iter().map(|e| e.get_ids().len()).max().unwrap_or(0);
            let mut batch = EncodedPairBatch {
                batch_size,
                seq_len,
                input_ids: Vec::with_capacity(batch_size * seq_len),
                attention_mask: Vec::with_capacity(batch_size * seq_len),
                token_type_ids: Vec::with_capacity(batch_size * seq_len),
            };
            for enc in &encodings {
                if enc.get_ids().len() != seq_len {
                    return Err("Tokenizer returned ragged batch despite padding".to_string());
                }
                batch.input_ids.extend(enc.get_ids().iter().map(|&v| v as i64));
                batch.attention_mask.extend(enc.get_attention_mask().iter().map(|&v| v as i64));
                batch.token_type_ids.extend(enc.get_type_ids().iter().map(|&v| v as i64));
            }
            Ok(batch)
        }
    }
}

#[cfg(not(feature = "runtime"))]
mod imp {
    use super::EncodedPairBatch;
    use std::path::Path;

    /// Placeholder tokenizer for builds compiled without the `runtime` feature.
    #[derive(Debug)]
    pub struct PairTokenizer {
        _private: (),
    }

    impl PairTokenizer {
        pub fn from_file(path: &Path, _max_length: usize) -> Result<Self, String> {
            Err(format!(
                "Tokenizers are not linked in this build (feature `runtime` disabled): {}",
                path.display()
            ))
        }

        pub fn max_length(&self) -> usize {
            0
        }

        pub fn encode_pairs(&self, _query: &str, _documents: &[String]) -> Result<EncodedPairBatch, String> {
            Err("Tokenizers are not linked in this build".to_string())
        }
    }
}

pub use imp::PairTokenizer;

/// Loads a pair tokenizer, returning a descriptive error instead of panicking.
pub fn load_pair_tokenizer(path: &Path, max_length: usize) -> Result<PairTokenizer, String> {
    PairTokenizer::from_file(path, max_length)
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use super::*;

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tiny_tokenizer.json")
    }

    #[test]
    fn test_pair_encoding_shapes_and_mask() {
        let tok = load_pair_tokenizer(&fixture(), 16).expect("fixture tokenizer loads");
        let docs = vec!["timeout is thirty seconds".to_string(), "theme".to_string()];
        let batch = tok.encode_pairs("what timeout", &docs).unwrap();
        assert_eq!(batch.batch_size, 2);
        assert_eq!(batch.input_ids.len(), batch.batch_size * batch.seq_len);
        assert_eq!(batch.attention_mask.len(), batch.input_ids.len());
        // Second pair is shorter, so it must be padded with mask zeros at the tail.
        let row2 = &batch.attention_mask[batch.seq_len..];
        assert_eq!(*row2.last().unwrap(), 0);
        assert_eq!(batch.attention_mask[0], 1);
    }

    #[test]
    fn test_pair_truncation_respects_max_length() {
        let tok = load_pair_tokenizer(&fixture(), 8).unwrap();
        let long_doc = "timeout ".repeat(50);
        let batch = tok.encode_pairs("what timeout", &[long_doc]).unwrap();
        assert!(batch.seq_len <= 8, "seq_len {} exceeds max_length", batch.seq_len);
    }

    #[test]
    fn test_missing_tokenizer_is_error() {
        assert!(load_pair_tokenizer(Path::new("missing_tokenizer.json"), 16).is_err());
    }
}
