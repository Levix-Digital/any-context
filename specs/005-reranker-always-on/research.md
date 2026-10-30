# Research: Reranker always on

## R1. What replaces `/rerank`

- **Decision**: remove the command with all its options. Its status view (model, state, depths, last pass) becomes one `Reranker` line in `/status`. Installing or repairing weights stays with `actx download reranker` / `/download reranker`.
- **Rationale**: the design (step 6) removes the command; the status is still useful for support and the manual tests, and `/status` already shows the other models and the dense search state.
- **Alternatives**: keep `/rerank` as status only (rejected: the backlog acceptance removes it, and a command named after a switch invites the question of how to switch it).

## R2. The reranker model choice

- **Decision**: `/rerank model <id>` goes away with the command. The `reranker_model` setting already stored is still honored, `--rerank-model` stays for one run (tests and comparisons), and the hardware kit chooses the model in IMP-055.
- **Rationale**: the design says the model comes from the hardware kit, with no user option; adding a new command now would be removed again by IMP-055.
- **Alternatives**: a `/reranker <id>` command for the model only (rejected for the reason above; it can be added if the user wants the choice back before IMP-055).

## R3. A reranker that cannot run

- **Decision**: keep the fusion order without discarding anything, and report a `RerankOutcome::Unavailable(cause)` in `RetrievalOutput`, turned into an answer notice like the dense outcome (ADR-0155). Causes: no reranker in the pipeline, ONNX Runtime missing, weights missing, load failure, scoring failure or wrong score count.
- **Rationale**: discarding everything would leave no answer on a machine whose reranker is broken; silently keeping RRF order would bring back the off mode without the user knowing. The notice tells the cause and the fix.
- **Alternatives**: fail the question (rejected: too harsh for a missing optional download on Lite); answer nothing found (rejected: the evidence may be good).

## R4. Discarding the tail

- **Decision**: after a successful pass, the result keeps the pinned candidates and the scored ones; `apply_neural_rerank` no longer rescales a tail.
- **Rationale**: IMP-043 acceptance. The selection then works only on judged candidates, which IMP-044 to IMP-046 need.
- **Effect on the benchmark**: none expected on MRR@10 and HitRate@5 (the depth is 30, the top 10 already came from scored candidates); HitRate@10 and nothing-found counts are compared in CI.

## Benchmark results

To be filled after the CI run.
