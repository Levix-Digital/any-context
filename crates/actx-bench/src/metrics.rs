//! Retrieval quality metrics over the per-question outcomes of a run.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::corpus::normalize_whitespace;

/// Depth of the ranking metrics (HitRate@10, MRR@10, NDCG@10).
pub const DEPTH: usize = 10;
/// Depth of the passage metric.
pub const PASSAGE_DEPTH: usize = 5;

/// A document of the ranking, at its best position.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RankedDoc {
    pub doc: String,
    pub score: f64,
}

/// What the pipeline returned for one question, before scoring.
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub id: String,
    pub lang: String,
    pub category: String,
    pub negative: bool,
    /// Relevant documents and their grades (1 to 3).
    pub relevant: Vec<(String, u8)>,
    /// Expected passages; empty when the question has none.
    pub passages: Vec<String>,
    /// Documents of the ranking pass in rank order, each once.
    pub ranked: Vec<RankedDoc>,
    /// Chunk texts of the ranking pass in rank order.
    pub ranked_chunks: Vec<String>,
    /// Documents of the delivered pass (the product's request as is).
    pub delivered: Vec<String>,
    pub latency_ms: f64,
}

/// The scored result of one question, as written in the result file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionResult {
    pub id: String,
    pub lang: String,
    pub category: String,
    /// 1-based rank of the first relevant document within the top 10; `None` when missing or negative.
    pub first_relevant_rank: Option<usize>,
    pub ndcg_at_10: f64,
    /// `None` when the question has no expected passage.
    pub passage_hit_at_5: Option<bool>,
    pub delivered_hit: bool,
    pub nothing_found: bool,
    pub ranked: Vec<RankedDoc>,
    /// Best reranker score of a relevant document in the top 10 of the ranking pass (which keeps
    /// irrelevant chunks), for the band calibration; `None` when negative or missing (IMP-044).
    #[serde(default)]
    pub best_relevant_score: Option<f64>,
}

impl QuestionResult {
    pub fn is_negative(&self) -> bool {
        self.category == "negative"
    }

    /// Best score of the ranking pass, relevant or not.
    pub fn top_score(&self) -> Option<f64> {
        self.ranked.first().map(|d| d.score)
    }

    pub fn hit_at(&self, k: usize) -> bool {
        self.first_relevant_rank.is_some_and(|r| r <= k)
    }
}

/// Aggregated metrics of a group of questions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Metrics {
    pub hit_rate_at_1: f64,
    pub hit_rate_at_3: f64,
    pub hit_rate_at_5: f64,
    pub hit_rate_at_10: f64,
    pub mrr_at_10: f64,
    pub ndcg_at_10: f64,
    pub passage_hit_rate_at_5: f64,
    pub delivered_hit_rate: f64,
    pub nothing_found_rate: f64,
    pub false_nothing_found_rate: f64,
    pub positives: usize,
    pub negatives: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MetricSet {
    pub overall: Metrics,
    pub by_lang: BTreeMap<String, Metrics>,
    pub by_category: BTreeMap<String, Metrics>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Latency {
    pub mean: f64,
    pub p95: f64,
    pub questions: usize,
}

/// Irrelevant thresholds the calibration table tries (IMP-044).
pub const CALIBRATION_THRESHOLDS: [f64; 5] = [0.005, 0.01, 0.02, 0.03, 0.05];

/// What one irrelevant threshold would do to the ranking pass of a run.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationRow {
    pub threshold: f64,
    /// Negatives whose best score is below the threshold: nothing would be delivered.
    pub negatives_empty: usize,
    pub negatives: usize,
    /// Positives with no relevant document at or above the threshold (or none in the top 10).
    pub positives_lost: usize,
    pub positives: usize,
}

/// The calibration table of the band thresholds over the scored questions (IMP-044).
pub fn calibration(results: &[QuestionResult]) -> Vec<CalibrationRow> {
    let (negatives, positives): (Vec<&QuestionResult>, Vec<&QuestionResult>) = results.iter().partition(|r| r.is_negative());
    CALIBRATION_THRESHOLDS
        .iter()
        .map(|&t| CalibrationRow {
            threshold: t,
            negatives_empty: negatives.iter().filter(|r| r.top_score().is_none_or(|s| s < t)).count(),
            negatives: negatives.len(),
            positives_lost: positives.iter().filter(|r| r.best_relevant_score.is_none_or(|s| s < t)).count(),
            positives: positives.len(),
        })
        .collect()
}

/// Scores one question.
pub fn score(outcome: &Outcome) -> QuestionResult {
    let grade = |doc: &str| outcome.relevant.iter().filter(|(d, _)| d == doc).map(|(_, g)| *g).max().unwrap_or(0);
    let top: Vec<&RankedDoc> = outcome.ranked.iter().take(DEPTH).collect();

    let (first_relevant_rank, ndcg) = if outcome.negative {
        (None, 0.0)
    } else {
        let rank = top.iter().position(|d| grade(&d.doc) > 0).map(|i| i + 1);
        let dcg: f64 = top.iter().enumerate().map(|(i, d)| gain(grade(&d.doc)) / discount(i)).sum();
        let mut ideal: Vec<u8> = outcome.relevant.iter().map(|(_, g)| *g).collect();
        ideal.sort_unstable_by(|a, b| b.cmp(a));
        let idcg: f64 = ideal.iter().take(DEPTH).enumerate().map(|(i, g)| gain(*g) / discount(i)).sum();
        (rank, if idcg > 0.0 { dcg / idcg } else { 0.0 })
    };

    let passage_hit_at_5 = (!outcome.passages.is_empty()).then(|| {
        let chunks: Vec<String> = outcome.ranked_chunks.iter().take(PASSAGE_DEPTH).map(|c| normalize_whitespace(c)).collect();
        outcome
            .passages
            .iter()
            .map(|p| normalize_whitespace(p))
            .any(|p| chunks.iter().any(|c| c.contains(&p)))
    });

    QuestionResult {
        id: outcome.id.clone(),
        lang: outcome.lang.clone(),
        category: outcome.category.clone(),
        first_relevant_rank,
        ndcg_at_10: round4(ndcg),
        passage_hit_at_5,
        delivered_hit: !outcome.negative && outcome.delivered.iter().any(|d| grade(d) > 0),
        nothing_found: outcome.delivered.is_empty(),
        best_relevant_score: if outcome.negative {
            None
        } else {
            top.iter().filter(|d| grade(&d.doc) > 0).map(|d| round4(d.score)).reduce(f64::max)
        },
        ranked: top.into_iter().map(|d| RankedDoc { doc: d.doc.clone(), score: round4(d.score) }).collect(),
    }
}

/// Aggregates scored questions overall, by language and by category.
pub fn aggregate(results: &[QuestionResult]) -> MetricSet {
    let mut by_lang: BTreeMap<String, Vec<&QuestionResult>> = BTreeMap::new();
    let mut by_category: BTreeMap<String, Vec<&QuestionResult>> = BTreeMap::new();
    for r in results {
        by_lang.entry(r.lang.clone()).or_default().push(r);
        by_category.entry(r.category.clone()).or_default().push(r);
    }
    MetricSet {
        overall: metrics(&results.iter().collect::<Vec<_>>()),
        by_lang: by_lang.into_iter().map(|(k, v)| (k, metrics(&v))).collect(),
        by_category: by_category.into_iter().map(|(k, v)| (k, metrics(&v))).collect(),
    }
}

fn metrics(results: &[&QuestionResult]) -> Metrics {
    let positives: Vec<&&QuestionResult> = results.iter().filter(|r| !r.is_negative()).collect();
    let negatives: Vec<&&QuestionResult> = results.iter().filter(|r| r.is_negative()).collect();
    let rate = |n: usize, d: usize| if d == 0 { 0.0 } else { round4(n as f64 / d as f64) };
    let mean = |sum: f64, d: usize| if d == 0 { 0.0 } else { round4(sum / d as f64) };
    let hits = |k: usize| rate(positives.iter().filter(|r| r.hit_at(k)).count(), positives.len());
    let with_passage: Vec<bool> = positives.iter().filter_map(|r| r.passage_hit_at_5).collect();

    Metrics {
        hit_rate_at_1: hits(1),
        hit_rate_at_3: hits(3),
        hit_rate_at_5: hits(5),
        hit_rate_at_10: hits(DEPTH),
        mrr_at_10: mean(positives.iter().filter_map(|r| r.first_relevant_rank).map(|r| 1.0 / r as f64).sum(), positives.len()),
        ndcg_at_10: mean(positives.iter().map(|r| r.ndcg_at_10).sum(), positives.len()),
        passage_hit_rate_at_5: rate(with_passage.iter().filter(|h| **h).count(), with_passage.len()),
        delivered_hit_rate: rate(positives.iter().filter(|r| r.delivered_hit).count(), positives.len()),
        nothing_found_rate: rate(negatives.iter().filter(|r| r.nothing_found).count(), negatives.len()),
        false_nothing_found_rate: rate(positives.iter().filter(|r| r.nothing_found).count(), positives.len()),
        positives: positives.len(),
        negatives: negatives.len(),
    }
}

/// Mean and nearest-rank p95 of the per-question latencies, in milliseconds.
pub fn latency(samples_ms: &[f64]) -> Latency {
    if samples_ms.is_empty() {
        return Latency::default();
    }
    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((0.95 * sorted.len() as f64).ceil() as usize).saturating_sub(1);
    Latency {
        mean: round4(sorted.iter().sum::<f64>() / sorted.len() as f64),
        p95: round4(sorted[idx]),
        questions: sorted.len(),
    }
}

/// Rounds to 4 decimals, so result files only change when a metric does.
pub fn round4(x: f64) -> f64 {
    (x * 10_000.0).round() / 10_000.0
}

fn gain(grade: u8) -> f64 {
    (2f64.powi(grade as i32)) - 1.0
}

fn discount(index: usize) -> f64 {
    ((index + 2) as f64).log2()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranked(docs: &[&str]) -> Vec<RankedDoc> {
        docs.iter().enumerate().map(|(i, d)| RankedDoc { doc: d.to_string(), score: 1.0 - i as f64 * 0.1 }).collect()
    }

    fn positive(id: &str, relevant: &[(&str, u8)], docs: &[&str]) -> Outcome {
        Outcome {
            id: id.into(),
            lang: "pt".into(),
            category: "factual".into(),
            relevant: relevant.iter().map(|(d, g)| (d.to_string(), *g)).collect(),
            ranked: ranked(docs),
            delivered: docs.iter().take(2).map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    fn negative(id: &str, delivered: &[&str]) -> Outcome {
        Outcome {
            id: id.into(),
            lang: "en".into(),
            category: "negative".into(),
            negative: true,
            delivered: delivered.iter().map(|d| d.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn the_best_relevant_score_comes_from_the_ranking_pass() {
        let r = score(&positive("q", &[("b", 3), ("c", 1)], &["a", "b", "c"]));
        assert_eq!(r.best_relevant_score, Some(0.9));
        assert_eq!(r.top_score(), Some(1.0));
        assert_eq!(score(&positive("q", &[("x", 3)], &["a"])).best_relevant_score, None);
        assert_eq!(score(&negative("n", &[])).best_relevant_score, None);
    }

    #[test]
    fn calibration_counts_empty_negatives_and_lost_positives() {
        let mut low = score(&positive("low", &[("a", 3)], &["a"]));
        low.best_relevant_score = Some(0.015);
        let high = score(&positive("high", &[("a", 3)], &["a"]));
        let missing = score(&positive("missing", &[("x", 3)], &["a"]));
        let mut weak = score(&negative("weak", &[]));
        weak.ranked = vec![RankedDoc { doc: "a".into(), score: 0.008 }];
        let empty = score(&negative("empty", &[]));
        let rows = calibration(&[low, high, missing, weak, empty]);
        assert_eq!(rows.len(), CALIBRATION_THRESHOLDS.len());
        let at = |t: f64| rows.iter().find(|r| r.threshold == t).unwrap().clone();
        assert_eq!(at(0.005), CalibrationRow { threshold: 0.005, negatives_empty: 1, negatives: 2, positives_lost: 1, positives: 3 });
        assert_eq!((at(0.01).negatives_empty, at(0.01).positives_lost), (2, 1));
        assert_eq!((at(0.02).negatives_empty, at(0.02).positives_lost), (2, 2));
    }

    #[test]
    fn rank_one_is_a_hit_everywhere() {
        let r = score(&positive("q", &[("a", 3)], &["a", "b"]));
        assert_eq!(r.first_relevant_rank, Some(1));
        assert!(r.hit_at(1) && r.hit_at(10));
        assert_eq!(r.ndcg_at_10, 1.0);
        assert!(r.delivered_hit && !r.nothing_found);
    }

    #[test]
    fn rank_three_counts_from_hit_at_3() {
        let results = vec![score(&positive("q", &[("c", 3)], &["a", "b", "c"]))];
        let m = aggregate(&results).overall;
        assert_eq!((m.hit_rate_at_1, m.hit_rate_at_3, m.hit_rate_at_5, m.hit_rate_at_10), (0.0, 1.0, 1.0, 1.0));
        assert_eq!(m.mrr_at_10, round4(1.0 / 3.0));
        assert_eq!(m.ndcg_at_10, round4(1.0 / 4f64.log2()));
        assert_eq!(m.delivered_hit_rate, 0.0, "only the first two documents were delivered");
    }

    #[test]
    fn beyond_ten_is_a_miss() {
        let docs: Vec<String> = (0..11).map(|i| format!("d{i}")).collect();
        let refs: Vec<&str> = docs.iter().map(String::as_str).collect();
        let r = score(&positive("q", &[("d10", 3)], &refs));
        assert_eq!(r.first_relevant_rank, None);
        assert_eq!(r.ndcg_at_10, 0.0);
        assert_eq!(r.ranked.len(), DEPTH, "the result file keeps the top 10 only");
        assert_eq!(aggregate(&[r]).overall.mrr_at_10, 0.0);
    }

    #[test]
    fn graded_ndcg_rewards_the_better_order() {
        let best = score(&positive("q", &[("full", 3), ("part", 1)], &["full", "part"]));
        let swapped = score(&positive("q", &[("full", 3), ("part", 1)], &["part", "full"]));
        assert_eq!(best.ndcg_at_10, 1.0);
        let expected = (1.0 + 7.0 / 3f64.log2()) / (7.0 + 1.0 / 3f64.log2());
        assert_eq!(swapped.ndcg_at_10, round4(expected));
        assert_eq!(swapped.first_relevant_rank, Some(1), "MRR uses the first relevant document of any grade");
    }

    #[test]
    fn fewer_than_ten_results_are_not_padded() {
        let r = score(&positive("q", &[("x", 3)], &["a"]));
        assert_eq!(r.first_relevant_rank, None);
        assert_eq!(r.ranked.len(), 1);
    }

    #[test]
    fn negatives_stay_out_of_ranking_metrics() {
        let results = vec![
            score(&positive("p", &[("a", 3)], &["a"])),
            score(&negative("n1", &[])),
            score(&negative("n2", &["a"])),
        ];
        let m = aggregate(&results).overall;
        assert_eq!((m.positives, m.negatives), (1, 2));
        assert_eq!((m.hit_rate_at_1, m.mrr_at_10), (1.0, 1.0));
        assert_eq!(m.nothing_found_rate, 0.5);
        assert_eq!(m.false_nothing_found_rate, 0.0);
        assert!(!results[2].delivered_hit && results[2].first_relevant_rank.is_none());
    }

    #[test]
    fn a_positive_with_nothing_delivered_is_a_false_nothing_found() {
        let mut o = positive("p", &[("a", 3)], &["a"]);
        o.delivered.clear();
        let m = aggregate(&[score(&o)]).overall;
        assert_eq!(m.false_nothing_found_rate, 1.0);
        assert_eq!(m.delivered_hit_rate, 0.0);
    }

    #[test]
    fn passage_hits_use_the_top_five_chunks_with_normalized_whitespace() {
        let mut o = positive("p", &[("a", 3)], &["a"]);
        o.passages = vec!["prazo de\n 30 dias".into()];
        o.ranked_chunks = vec!["x".into(), "x".into(), "x".into(), "x".into(), "o prazo de 30   dias corridos".into()];
        assert_eq!(score(&o).passage_hit_at_5, Some(true));
        o.ranked_chunks.insert(0, "y".into());
        assert_eq!(score(&o).passage_hit_at_5, Some(false));
        o.passages.clear();
        assert_eq!(score(&o).passage_hit_at_5, None);
    }

    #[test]
    fn groups_by_language_and_category() {
        let results = vec![score(&positive("p", &[("a", 3)], &["a"])), score(&negative("n", &[]))];
        let set = aggregate(&results);
        assert_eq!(set.by_lang["pt"].positives, 1);
        assert_eq!(set.by_lang["en"].negatives, 1);
        assert_eq!(set.by_category["negative"].nothing_found_rate, 1.0);
        assert_eq!(set.by_category["factual"].hit_rate_at_1, 1.0);
    }

    #[test]
    fn latency_mean_and_nearest_rank_p95() {
        let samples: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let l = latency(&samples);
        assert_eq!((l.mean, l.p95, l.questions), (10.5, 19.0, 20));
        assert_eq!(latency(&[]), Latency::default());
    }

    #[test]
    fn rounding_keeps_four_decimals() {
        assert_eq!(round4(1.0 / 3.0), 0.3333);
        assert_eq!(round4(0.66666), 0.6667);
    }
}
