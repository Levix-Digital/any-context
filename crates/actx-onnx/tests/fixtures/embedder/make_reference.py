"""Reference vectors for the ONNX embedder test (FEAT-067, T011).

Embeds six Portuguese and English sentences with the same INT8 export AnyContext downloads, using
the Python `onnxruntime` and `tokenizers` packages and mean pooling under the attention mask
followed by L2 normalization, all six texts in one padded batch (as the Rust test embeds them),
and writes them to `e5-small-reference.json`. The Rust embedder
must reproduce these vectors (cosine >= 0.9999).

Usage:
    pip install onnxruntime==1.31.0 tokenizers numpy

The onnxruntime version must be the one AnyContext ships (scripts/fetch_onnxruntime.sh): the INT8
kernels differ between releases by about 0.002 in cosine.
    python make_reference.py <dir with model.onnx and tokenizer.json>
"""

import json
import pathlib
import sys

import numpy as np
import onnxruntime as ort
from tokenizers import Tokenizer

MODEL_ID = "multilingual-e5-small"
MAX_TOKENS = 512
TEXTS = [
    "query: Qual é o timeout padrão do serviço de pagamento v2?",
    "passage: No serviço de pagamento v2, o timeout padrão de requisições síncronas é de 30 segundos.",
    "passage: O tema visual da interface usa gradientes azul e roxo.",
    "query: How do I rotate the database credentials?",
    "passage: Database credentials are rotated every 90 days by the platform team using the vault CLI.",
    "passage: The quarterly report lists revenue by region and product line.",
]


def main(model_dir: pathlib.Path) -> None:
    session = ort.InferenceSession(str(model_dir / "model.onnx"), providers=["CPUExecutionProvider"])
    tokenizer = Tokenizer.from_file(str(model_dir / "tokenizer.json"))
    tokenizer.enable_truncation(MAX_TOKENS)
    # Pad with the model's own pad token, as the Rust tokenizer does: the graph derives position
    # ids from it and the INT8 activations are quantized over the whole batch, so a different pad
    # id (or batch) changes the vectors of the real tokens too.
    tokenizer.enable_padding(pad_id=tokenizer.token_to_id("<pad>"), pad_token="<pad>")
    encodings = tokenizer.encode_batch(TEXTS)
    ids = np.array([e.ids for e in encodings], dtype=np.int64)
    mask = np.array([e.attention_mask for e in encodings], dtype=np.int64)
    feeds = {"input_ids": ids, "attention_mask": mask}
    if any(i.name == "token_type_ids" for i in session.get_inputs()):
        feeds["token_type_ids"] = np.zeros_like(ids)
    states = session.run(None, feeds)[0]
    summed = (states * mask[:, :, None]).sum(axis=1)
    vectors = summed / mask.sum(axis=1, keepdims=True)
    vectors /= np.linalg.norm(vectors, axis=1, keepdims=True)
    out = {
        "model_id": MODEL_ID,
        "pooling": "mean",
        "texts": TEXTS,
        "vectors": [[round(float(x), 7) for x in v] for v in vectors],
    }
    path = pathlib.Path(__file__).with_name("e5-small-reference.json")
    path.write_text(json.dumps(out, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {path} ({len(TEXTS)} vectors of {vectors.shape[1]} dimensions)")


if __name__ == "__main__":
    main(pathlib.Path(sys.argv[1]))
