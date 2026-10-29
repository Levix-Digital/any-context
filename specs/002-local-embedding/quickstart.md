# Quickstart: local embedding (spec 002)

```bash
# ONNX Runtime for local runs (same as CI)
bash scripts/fetch_onnxruntime.sh .ort && export ORT_DYLIB_PATH=$PWD/.ort/libonnxruntime.so

# Unit and integration tests (local embedder tests download the e5-small files once into ACTX_MODELS_DIR)
CARGO_PROFILE_DEV_DEBUG=0 cargo test --workspace

# Benchmark with each candidate (no API keys)
cargo run -p actx-bench --release -- retrieval run --embedding local/multilingual-e5-small
cargo run -p actx-bench --release -- retrieval run --embedding local/multilingual-e5-base
cargo run -p actx-bench --release -- retrieval run --embedding local/bge-m3

# Product: see and change the embedding model
actx embedding
actx embedding multilingual-e5-base --yes
```
