# Data model: Development retrieval benchmark

All files live under `benchmarks/retrieval/` and are plain JSON or documents. Nothing is stored in a product database.

## Question file (`questions.json`)

```json
{
  "questions": [
    {
      "id": "pt-pay-003",
      "lang": "pt",
      "category": "version",
      "text": "Qual é o timeout padrão das requisições síncronas no serviço de pagamento v2?",
      "relevant": [
        { "doc": "pt/pagamentos/servico-pagamento-v2.md", "grade": 3,
          "passage": "o timeout padrão de requisições síncronas é de 30 segundos" }
      ]
    },
    { "id": "en-neg-004", "lang": "en", "category": "negative",
      "text": "What is the parental leave policy?", "relevant": [] }
  ]
}
```

| Field | Rule |
|---|---|
| `id` | unique, `<lang>-<topic>-<nnn>` |
| `lang` | `pt` or `en` |
| `category` | `factual`, `version`, `form-field`, `cross-language`, `negative` |
| `text` | non-empty |
| `relevant` | empty if and only if `category` is `negative` |
| `relevant[].doc` | path relative to `corpus/`; the file must exist |
| `relevant[].grade` | 1 (related), 2 (partial answer), 3 (full answer) |
| `relevant[].passage` | optional; must be a substring of the document's extracted text (whitespace normalized) |

Validation (FR-005) runs before indexing and lists every broken question.

## Run configuration

| Field | Example |
|---|---|
| `config_id` | `lexical-rerank` (CI) or `dense-<provider>-<model>-rerank` |
| `corpus_version` | SHA-256 of sorted relative paths and contents of `corpus/` |
| `questions_version` | SHA-256 of `questions.json` |
| `retrievers` | `{ "lexical": true, "dense": false, "reranker": true }` |
| `embedding_target` | `mock/mock-embed` or e.g. `gemini/gemini-embedding-2` |
| `reranker_model` | catalog id and SHA-256 (`mmarco-reranker`) |
| `code_revision` | `git rev-parse HEAD` when available, else `unknown` |

Two runs are **comparable** when `config_id`, `corpus_version`, `questions_version`, `embedding_target` and `reranker_model` are equal.

## Run result (`results/<config_id>.json`, baseline `baseline.<config_id>.json`)

```json
{
  "schema": 1,
  "config": { "...": "run configuration" },
  "metrics": {
    "overall": { "hit_rate_at_1": 0.0, "hit_rate_at_3": 0.0, "hit_rate_at_5": 0.0,
                 "hit_rate_at_10": 0.0, "mrr_at_10": 0.0, "ndcg_at_10": 0.0,
                 "passage_hit_rate_at_5": 0.0, "delivered_hit_rate": 0.0,
                 "nothing_found_rate": 0.0, "false_nothing_found_rate": 0.0,
                 "positives": 0, "negatives": 0 },
    "by_lang": { "pt": { "...": "same fields" }, "en": {} },
    "by_category": { "version": {} }
  },
  "latency_ms": { "mean": 0.0, "p95": 0.0, "questions": 0 },
  "questions": [
    { "id": "pt-pay-003", "first_relevant_rank": 2, "passage_hit_at_5": true,
      "delivered_hit": true, "nothing_found": false,
      "ranked": [ { "doc": "pt/pagamentos/servico-pagamento-v1.md", "score": 0.91 } ] }
  ]
}
```

Floats are rounded to 4 decimals when written, so the file diff in a pull request shows real changes only. Latency is kept out of `metrics` so that it never affects comparability or the gate.

## Comparison

Per metric of `overall`: `baseline`, `current`, `delta`; plus `regressed` (hit at 5 → miss at 5) and `improved` question ids. `failed` is true when `mrr_at_10` or `hit_rate_at_5` delta < −0.02. Non-comparable runs give `comparable: false` with the differing fields and no deltas.
