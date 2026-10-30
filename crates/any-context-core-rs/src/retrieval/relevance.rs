//! Relevance bands read from the cross-encoder score (IMP-044).
//!
//! Each reranker in the model catalog carries two thresholds calibrated on the retrieval
//! benchmark. A scored chunk is relevant, maybe or irrelevant; irrelevant chunks never reach the
//! selection. Per-workspace refinement by the evaluation set is IMP-008's.

use serde::{Deserialize, Serialize};

/// How relevant the cross-encoder judged a chunk to its question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelevanceBand {
    Relevant,
    Maybe,
    Irrelevant,
}

/// The two thresholds of one reranker model; a score at a threshold takes the higher band.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RelevanceThresholds {
    /// Scores below this are irrelevant.
    pub irrelevant_below: f32,
    /// Scores from this up are relevant; between the two, maybe.
    pub relevant_from: f32,
}

// The thresholds are finite catalog constants (never NaN), so equality is total.
impl Eq for RelevanceThresholds {}

impl RelevanceThresholds {
    pub fn band(&self, score: f32) -> RelevanceBand {
        if score >= self.relevant_from {
            RelevanceBand::Relevant
        } else if score >= self.irrelevant_below {
            RelevanceBand::Maybe
        } else {
            RelevanceBand::Irrelevant
        }
    }

    /// Both thresholds inside the sigmoid range and in order.
    pub fn is_valid(&self) -> bool {
        0.0 < self.irrelevant_below && self.irrelevant_below < self.relevant_from && self.relevant_from < 1.0
    }
}

/// How many candidates of one reranking pass fell in each band; the irrelevant ones were discarded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BandCounts {
    pub relevant: usize,
    pub maybe: usize,
    pub irrelevant: usize,
}

impl BandCounts {
    pub fn add(&mut self, band: RelevanceBand) {
        match band {
            RelevanceBand::Relevant => self.relevant += 1,
            RelevanceBand::Maybe => self.maybe += 1,
            RelevanceBand::Irrelevant => self.irrelevant += 1,
        }
    }

    /// `<r> relevant / <m> maybe / <i> irrelevant`, for status surfaces.
    pub fn describe(&self) -> String {
        format!("{} relevant / {} maybe / {} irrelevant", self.relevant, self.maybe, self.irrelevant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: RelevanceThresholds = RelevanceThresholds { irrelevant_below: 0.005, relevant_from: 0.6 };

    #[test]
    fn a_score_at_a_threshold_takes_the_higher_band() {
        assert_eq!(T.band(0.6), RelevanceBand::Relevant);
        assert_eq!(T.band(0.005), RelevanceBand::Maybe);
    }

    #[test]
    fn scores_around_the_thresholds() {
        assert_eq!(T.band(0.99), RelevanceBand::Relevant);
        assert_eq!(T.band(0.5999), RelevanceBand::Maybe);
        assert_eq!(T.band(0.3), RelevanceBand::Maybe);
        assert_eq!(T.band(0.0049), RelevanceBand::Irrelevant);
        assert_eq!(T.band(0.0), RelevanceBand::Irrelevant);
    }

    #[test]
    fn validity_needs_ordered_thresholds_inside_the_range() {
        assert!(T.is_valid());
        assert!(!RelevanceThresholds { irrelevant_below: 0.6, relevant_from: 0.005 }.is_valid());
        assert!(!RelevanceThresholds { irrelevant_below: 0.0, relevant_from: 0.6 }.is_valid());
        assert!(!RelevanceThresholds { irrelevant_below: 0.1, relevant_from: 1.0 }.is_valid());
    }

    #[test]
    fn counts_add_up_per_band() {
        let mut c = BandCounts::default();
        for s in [0.9, 0.7, 0.1, 0.001] {
            c.add(T.band(s));
        }
        assert_eq!(c, BandCounts { relevant: 2, maybe: 1, irrelevant: 1 });
        assert_eq!(c.describe(), "2 relevant / 1 maybe / 1 irrelevant");
    }
}
