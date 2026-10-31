//! Comparison of a run with the baseline, the regression gate and the Markdown summary.

use crate::metrics::{calibration, CalibrationRow, Metrics, PASSAGE_DEPTH};
use crate::result::RunResult;

/// A drop larger than this in a gated metric fails the run (decided 2026-10-25).
pub const MAX_DROP: f64 = 0.02;

#[derive(Debug, Clone, PartialEq)]
pub struct Delta {
    pub metric: &'static str,
    pub baseline: f64,
    pub current: f64,
    pub delta: f64,
    pub gated: bool,
}

impl Delta {
    pub fn failed(&self) -> bool {
        self.gated && self.delta < -MAX_DROP - 1e-9
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    /// The configurations differ: deltas would compare different things.
    NotComparable { fields: Vec<&'static str> },
    Compared {
        deltas: Vec<Delta>,
        /// Hit at 5 in the baseline, miss now.
        regressed: Vec<String>,
        /// Miss at 5 in the baseline, hit now.
        improved: Vec<String>,
    },
}

impl Comparison {
    pub fn failed(&self) -> bool {
        matches!(self, Comparison::Compared { deltas, .. } if deltas.iter().any(Delta::failed))
    }
}

pub fn compare(baseline: &RunResult, current: &RunResult) -> Comparison {
    let fields = current.config.differences(&baseline.config);
    if !fields.is_empty() {
        return Comparison::NotComparable { fields };
    }
    let (b, c) = (&baseline.metrics.overall, &current.metrics.overall);
    let deltas = rows(b, c);
    let hit5 = |r: &RunResult, id: &str| r.questions.iter().find(|q| q.id == id).map(|q| q.hit_at(PASSAGE_DEPTH));
    let mut regressed = Vec::new();
    let mut improved = Vec::new();
    for q in current.questions.iter().filter(|q| !q.is_negative()) {
        match (hit5(baseline, &q.id), q.hit_at(PASSAGE_DEPTH)) {
            (Some(true), false) => regressed.push(q.id.clone()),
            (Some(false), true) => improved.push(q.id.clone()),
            _ => {}
        }
    }
    Comparison::Compared { deltas, regressed, improved }
}

fn rows(b: &Metrics, c: &Metrics) -> Vec<Delta> {
    let row = |metric, baseline: f64, current: f64, gated| Delta {
        metric,
        baseline,
        current,
        delta: crate::metrics::round4(current - baseline),
        gated,
    };
    vec![
        row("hit_rate_at_1", b.hit_rate_at_1, c.hit_rate_at_1, false),
        row("hit_rate_at_3", b.hit_rate_at_3, c.hit_rate_at_3, false),
        row("hit_rate_at_5", b.hit_rate_at_5, c.hit_rate_at_5, true),
        row("hit_rate_at_10", b.hit_rate_at_10, c.hit_rate_at_10, false),
        row("mrr_at_10", b.mrr_at_10, c.mrr_at_10, true),
        row("ndcg_at_10", b.ndcg_at_10, c.ndcg_at_10, false),
        row("passage_hit_rate_at_5", b.passage_hit_rate_at_5, c.passage_hit_rate_at_5, false),
        row("delivered_hit_rate", b.delivered_hit_rate, c.delivered_hit_rate, true),
        row("nothing_found_rate", b.nothing_found_rate, c.nothing_found_rate, false),
        row("false_nothing_found_rate", b.false_nothing_found_rate, c.false_nothing_found_rate, false),
    ]
}

/// Markdown report of a run, with the comparison when there is one.
pub fn summary(current: &RunResult, comparison: Option<&Comparison>) -> String {
    let cfg = &current.config;
    let m = &current.metrics.overall;
    let mut out = format!(
        "## Retrieval benchmark: `{}`\n\n{} positive and {} negative questions; embedding `{}`, reranker `{}`, revision `{}`.\n\n",
        cfg.config_id, m.positives, m.negatives, cfg.embedding_target, cfg.reranker_model, cfg.code_revision
    );
    match comparison {
        Some(Comparison::Compared { deltas, regressed, improved }) => {
            out.push_str("| metric | baseline | current | delta |\n|---|---:|---:|---:|\n");
            for d in deltas {
                let mark = if d.failed() { " ❌" } else if d.gated { " (gate)" } else { "" };
                out.push_str(&format!("| {}{} | {:.4} | {:.4} | {:+.4} |\n", d.metric, mark, d.baseline, d.current, d.delta));
            }
            out.push_str(&format!("\nRegressed at 5: {}\n\nImproved at 5: {}\n", list(regressed), list(improved)));
            let verdict = if comparison.is_some_and(Comparison::failed) {
                format!("\n**Regression**: a gated metric (MRR@10, HitRate@5 or the delivered hit rate) dropped more than {MAX_DROP}.\n")
            } else {
                "\nNo regression.\n".to_string()
            };
            out.push_str(&verdict);
        }
        Some(Comparison::NotComparable { fields }) => {
            out.push_str(&format!("**Not comparable** with the baseline (different {}). Update the baseline with `--save-baseline`.\n\n", fields.join(", ")));
            out.push_str(&metrics_table(m));
        }
        None => {
            out.push_str("No baseline to compare with.\n\n");
            out.push_str(&metrics_table(m));
        }
    }
    out.push_str(&calibration_table(&calibration(&current.questions)));
    out.push_str(&format!(
        "\nLatency (delivered pass, reported only): mean {:.1} ms, p95 {:.1} ms over {} questions.\n",
        current.latency_ms.mean, current.latency_ms.p95, current.latency_ms.questions
    ));
    out
}

/// What each irrelevant threshold would cut on the ranking pass (IMP-044).
fn calibration_table(rows: &[CalibrationRow]) -> String {
    let mut out = "\nBand calibration (ranking pass, reranker scores):\n\n| irrelevant below | negatives with nothing | positives that lose the relevant document |\n|---:|---:|---:|\n".to_string();
    for r in rows {
        out.push_str(&format!("| {} | {} / {} | {} / {} |\n", r.threshold, r.negatives_empty, r.negatives, r.positives_lost, r.positives));
    }
    out
}

fn metrics_table(m: &Metrics) -> String {
    let rows = rows(m, m);
    let mut out = "| metric | value |\n|---|---:|\n".to_string();
    for d in rows {
        out.push_str(&format!("| {} | {:.4} |\n", d.metric, d.current));
    }
    out
}

fn list(ids: &[String]) -> String {
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{Latency, MetricSet, QuestionResult};
    use crate::result::{Retrievers, RunConfig, RunResult, SCHEMA};

    fn question(id: &str, rank: Option<usize>) -> QuestionResult {
        QuestionResult {
            id: id.into(),
            lang: "pt".into(),
            category: "factual".into(),
            first_relevant_rank: rank,
            ndcg_at_10: 0.0,
            passage_hit_at_5: None,
            delivered_hit: false,
            nothing_found: false,
            ranked: vec![],
            best_relevant_score: None,
        }
    }

    fn run(mrr: f64, hit5: f64, questions: Vec<QuestionResult>) -> RunResult {
        let mut metrics = MetricSet::default();
        metrics.overall.mrr_at_10 = mrr;
        metrics.overall.hit_rate_at_5 = hit5;
        RunResult {
            schema: SCHEMA,
            config: RunConfig {
                config_id: "lexical-rerank".into(),
                corpus_version: "c1".into(),
                questions_version: "q1".into(),
                retrievers: Retrievers { lexical: true, dense: false, reranker: true },
                embedding_target: "mock/mock-embed".into(),
                reranker_model: "mmarco-reranker".into(),
                code_revision: "abc".into(),
            },
            metrics,
            latency_ms: Latency::default(),
            questions,
        }
    }

    #[test]
    fn the_gate_allows_a_drop_of_exactly_two_points() {
        let base = run(0.8, 0.9, vec![]);
        for (mrr, hit5, failed) in [
            (0.7801, 0.9, false),
            (0.78, 0.9, false),
            (0.7799, 0.9, true),
            (0.8, 0.8801, false),
            (0.8, 0.88, false),
            (0.8, 0.8799, true),
        ] {
            let cmp = compare(&base, &run(mrr, hit5, vec![]));
            assert_eq!(cmp.failed(), failed, "mrr {mrr} hit5 {hit5}");
        }
    }

    /// IMP-044: the relevance cut must not drop the relevant documents the answer receives.
    #[test]
    fn a_delivered_hit_rate_drop_fails_the_run() {
        let mut base = run(0.8, 0.9, vec![]);
        base.metrics.overall.delivered_hit_rate = 1.0;
        for (delivered, failed) in [(0.98, false), (0.97, true)] {
            let mut current = run(0.8, 0.9, vec![]);
            current.metrics.overall.delivered_hit_rate = delivered;
            assert_eq!(compare(&base, &current).failed(), failed, "delivered {delivered}");
        }
    }

    #[test]
    fn ungated_metrics_and_latency_never_fail() {
        let base = run(0.8, 0.9, vec![]);
        let mut current = run(0.8, 0.9, vec![]);
        current.metrics.overall.hit_rate_at_1 = 0.1;
        current.metrics.overall.ndcg_at_10 = 0.1;
        current.latency_ms = Latency { mean: 9_999.0, p95: 99_999.0, questions: 1 };
        current.config.code_revision = "other".into();
        assert!(!compare(&base, &current).failed());
    }

    #[test]
    fn each_configuration_field_makes_runs_not_comparable() {
        let base = run(0.8, 0.9, vec![]);
        let edits: Vec<(&str, fn(&mut RunConfig))> = vec![
            ("config_id", |c| c.config_id = "dense".into()),
            ("corpus_version", |c| c.corpus_version = "c2".into()),
            ("questions_version", |c| c.questions_version = "q2".into()),
            ("retrievers", |c| c.retrievers.dense = true),
            ("embedding_target", |c| c.embedding_target = "gemini/gemini-embedding-2".into()),
            ("reranker_model", |c| c.reranker_model = "bge-reranker-v2-m3".into()),
        ];
        for (field, edit) in edits {
            let mut current = run(0.1, 0.1, vec![]);
            edit(&mut current.config);
            let cmp = compare(&base, &current);
            assert_eq!(cmp, Comparison::NotComparable { fields: vec![field] });
            assert!(!cmp.failed(), "not comparable is reported, never a regression");
        }
    }

    #[test]
    fn lists_regressed_and_improved_questions() {
        let base = run(0.5, 0.5, vec![question("a", Some(1)), question("b", None), question("c", Some(2)), question("d", Some(6))]);
        let current = run(0.5, 0.5, vec![question("a", Some(6)), question("b", Some(3)), question("c", Some(1)), question("d", Some(5))]);
        let Comparison::Compared { regressed, improved, deltas } = compare(&base, &current) else {
            panic!("comparable");
        };
        assert_eq!(regressed, vec!["a"]);
        assert_eq!(improved, vec!["b", "d"]);
        assert!(deltas.iter().all(|d| d.delta == 0.0));
    }

    #[test]
    fn summary_names_the_failing_metric() {
        let base = run(0.8, 0.9, vec![question("a", Some(1))]);
        let current = run(0.7, 0.9, vec![question("a", None)]);
        let cmp = compare(&base, &current);
        let text = summary(&current, Some(&cmp));
        assert!(text.contains("| mrr_at_10 ❌ | 0.8000 | 0.7000 | -0.1000 |"), "{text}");
        assert!(text.contains("Regressed at 5: a"), "{text}");
        assert!(text.contains("**Regression**"), "{text}");
        let other = summary(&current, Some(&Comparison::NotComparable { fields: vec!["corpus_version"] }));
        assert!(other.contains("Not comparable") && other.contains("corpus_version"));
    }
}
