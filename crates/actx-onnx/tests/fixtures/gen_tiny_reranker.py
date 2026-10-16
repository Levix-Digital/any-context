"""Generates the tiny cross-encoder fixtures used by `actx-onnx` unit tests (ADR-130).

Outputs (written next to this script):
  - tiny_tokenizer.json : WordLevel tokenizer with RoBERTa-style pair template
                          (<s> A </s> </s> B </s>), whitespace pre-tokenizer.
  - tiny_reranker.onnx  : graph with inputs `input_ids` / `attention_mask` (int64, [batch, seq])
                          and output `logits` (float32, [batch, 1]).
                          logit = sum over unmasked tokens of a per-token weight, so documents
                          containing "timeout" (+2.0) and "seconds" (+1.0) outrank "theme" (-1.0).

The fixtures are deterministic and only exercise the inference plumbing (tokenization,
batching, padding, tensor I/O, sigmoid); they carry no semantic quality.

Regenerate with:  python gen_tiny_reranker.py   (requires `onnx` and `tokenizers`)
"""

from pathlib import Path

import numpy as np
import onnx
from onnx import TensorProto, helper, numpy_helper
from tokenizers import Tokenizer, models, pre_tokenizers, processors

HERE = Path(__file__).resolve().parent

VOCAB = [
    "<pad>", "<s>", "</s>", "<unk>",
    "what", "timeout", "is", "thirty", "seconds", "theme", "colors", "the", "v2", "legacy",
]
WEIGHTS = {"timeout": 2.0, "seconds": 1.0, "thirty": 0.5, "theme": -1.0, "colors": -1.0, "legacy": -0.5}


def build_tokenizer() -> None:
    vocab = {tok: i for i, tok in enumerate(VOCAB)}
    tok = Tokenizer(models.WordLevel(vocab=vocab, unk_token="<unk>"))
    tok.pre_tokenizer = pre_tokenizers.Whitespace()
    tok.post_processor = processors.TemplateProcessing(
        single="<s> $A </s>",
        pair="<s> $A </s> </s> $B </s>",
        special_tokens=[("<s>", vocab["<s>"]), ("</s>", vocab["</s>"])],
    )
    tok.save(str(HERE / "tiny_tokenizer.json"))


def build_model() -> None:
    emb = np.zeros((len(VOCAB), 1), dtype=np.float32)
    for tok, w in WEIGHTS.items():
        emb[VOCAB.index(tok), 0] = w

    nodes = [
        helper.make_node("Gather", ["embedding", "input_ids"], ["tok_w"], axis=0),          # [B,S,1]
        helper.make_node("Cast", ["attention_mask"], ["mask_f"], to=TensorProto.FLOAT),     # [B,S]
        helper.make_node("Unsqueeze", ["mask_f", "axis_last"], ["mask_3d"]),                # [B,S,1]
        helper.make_node("Mul", ["tok_w", "mask_3d"], ["masked"]),                          # [B,S,1]
        helper.make_node("ReduceSum", ["masked", "axis_seq"], ["logits"], keepdims=0),      # [B,1]
    ]
    graph = helper.make_graph(
        nodes,
        "tiny_reranker",
        inputs=[
            helper.make_tensor_value_info("input_ids", TensorProto.INT64, ["batch", "seq"]),
            helper.make_tensor_value_info("attention_mask", TensorProto.INT64, ["batch", "seq"]),
        ],
        outputs=[helper.make_tensor_value_info("logits", TensorProto.FLOAT, ["batch", 1])],
        initializer=[
            numpy_helper.from_array(emb, "embedding"),
            numpy_helper.from_array(np.array([2], dtype=np.int64), "axis_last"),
            numpy_helper.from_array(np.array([1], dtype=np.int64), "axis_seq"),
        ],
    )
    model = helper.make_model(graph, opset_imports=[helper.make_opsetid("", 17)], producer_name="actx-fixture")
    model.ir_version = 8
    onnx.checker.check_model(model)
    onnx.save(model, str(HERE / "tiny_reranker.onnx"))


if __name__ == "__main__":
    build_tokenizer()
    build_model()
    print("fixtures written to", HERE)
