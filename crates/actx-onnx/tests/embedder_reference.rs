//! The ONNX embedder reproduces the reference vectors of `multilingual-e5-small` computed with the
//! Python `onnxruntime` and `tokenizers` packages (`fixtures/embedder/make_reference.py`).
//!
//! Needs the model files and the ONNX Runtime library, so it is skipped with a message when they
//! are absent; CI sets `ACTX_REQUIRE_EMBEDDER_TESTS=1` to make it mandatory:
//!
//! ```text
//! ORT_DYLIB_PATH=<onnxruntime library> ACTX_MODELS_DIR=<models dir with multilingual-e5-small/> \
//!   cargo test -p actx-onnx --features runtime --test embedder_reference
//! ```

use std::path::PathBuf;

use actx_onnx::embedder::{model_files, EmbedderConfig, EmbeddingPooling, OnnxEmbedder};

#[derive(serde::Deserialize)]
struct Reference {
    model_id: String,
    texts: Vec<String>,
    vectors: Vec<Vec<f32>>,
}

fn model_dir() -> Option<PathBuf> {
    let dir = std::env::var("ACTX_MODELS_DIR").ok()?;
    let dir = PathBuf::from(dir).join("multilingual-e5-small");
    let (model, tokenizer) = model_files(&dir);
    (model.is_file() && tokenizer.is_file()).then_some(dir)
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (na * nb)
}

#[test]
fn e5_small_matches_the_reference_vectors() {
    let required = std::env::var("ACTX_REQUIRE_EMBEDDER_TESTS").as_deref() == Ok("1");
    let runtime = actx_onnx::runtime::runtime_library_present();
    let Some(dir) = model_dir().filter(|_| runtime) else {
        assert!(!required, "ACTX_REQUIRE_EMBEDDER_TESTS=1 but the model files or the ONNX Runtime library are missing");
        eprintln!("skipped: set ACTX_MODELS_DIR (with multilingual-e5-small/) and ORT_DYLIB_PATH, build with --features runtime");
        return;
    };

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/embedder/e5-small-reference.json");
    let reference: Reference = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert_eq!(reference.model_id, "multilingual-e5-small");

    let (model_path, tokenizer_path) = model_files(&dir);
    let embedder = OnnxEmbedder::new(EmbedderConfig {
        model_id: reference.model_id.clone(),
        model_path,
        tokenizer_path,
        dimension: 384,
        max_tokens: 512,
        pooling: EmbeddingPooling::Mean,
    });
    let vectors = embedder.embed(&reference.texts).expect("real inference");
    assert_eq!(vectors.len(), reference.vectors.len());
    for (i, (got, want)) in vectors.iter().zip(&reference.vectors).enumerate() {
        assert_eq!(got.len(), 384);
        let c = cosine(got, want);
        assert!(c >= 0.9999, "text {i}: cosine {c} with the reference");
    }
    assert_eq!(embedder.truncated_inputs(), 0);

    // The INT8 export quantizes activations over the whole batch, so a text embedded alone lands
    // very close to, but not exactly on, its vector in a batch.
    let single = embedder.embed(&reference.texts[..1]).unwrap();
    let c = cosine(&single[0], &vectors[0]);
    assert!(c >= 0.995, "single vs batch cosine {c}");
}

