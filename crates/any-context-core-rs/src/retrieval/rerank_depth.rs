//! How many fused candidates the cross-encoder scores in one search (IMP-042).
//!
//! The depth comes from the search mode and the hardware profile in effect, and drops when the
//! reranking cost measured on this machine would exceed the mode's time budget. Per-workspace
//! tuning by the evaluation set is FEAT-105's.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::hardware::ProfileTier;

/// Fewest candidates a time budget may leave to the reranker.
pub const MIN_RERANK_DEPTH: usize = 10;

/// Reranker slots one document (file or page) takes before its other chunks wait in reserve.
pub const PER_DOCUMENT_RERANK_SLOTS: usize = 3;

/// Depth of requests built without a profile (and of the standard profile in Fast).
pub const DEFAULT_RERANK_DEPTH: usize = 30;

/// Fast answers one question in one pass; Deep fuses several sub-questions per turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Fast,
    Deep,
}

/// Depth of `mode` on `tier`, before any time budget.
pub fn profile_depth(tier: ProfileTier, mode: SearchMode) -> usize {
    match (tier, mode) {
        (ProfileTier::Lite, SearchMode::Fast) => 20,
        (ProfileTier::Lite, SearchMode::Deep) => 30,
        (ProfileTier::SmartBalanced | ProfileTier::AirGappedPower, SearchMode::Fast) => 30,
        (ProfileTier::SmartBalanced | ProfileTier::AirGappedPower, SearchMode::Deep) => 50,
        (ProfileTier::WorkstationPro, SearchMode::Fast) => 40,
        (ProfileTier::WorkstationPro, SearchMode::Deep) => 75,
        (ProfileTier::SovereignUltra, SearchMode::Fast) => 50,
        (ProfileTier::SovereignUltra, SearchMode::Deep) => 100,
    }
}

/// Time one search of `mode` may spend in the cross-encoder, in milliseconds.
pub fn time_budget_ms(mode: SearchMode) -> u64 {
    match mode {
        SearchMode::Fast => 1_000,
        SearchMode::Deep => 3_000,
    }
}

/// `depth` lowered so `cost_us` per candidate fits in `budget_ms`, never below
/// [`MIN_RERANK_DEPTH`] (nor above `depth`).
pub fn depth_within_budget(depth: usize, budget_ms: u64, cost_us: u64) -> usize {
    if cost_us == 0 {
        return depth;
    }
    let fits = (budget_ms.saturating_mul(1_000) / cost_us) as usize;
    fits.clamp(MIN_RERANK_DEPTH.min(depth), depth)
}

/// Measured reranking cost per candidate on this machine, in microseconds (0 = not measured).
static COST_PER_CANDIDATE_US: AtomicU64 = AtomicU64::new(0);

/// Records a reranking pass of `candidates` that took `elapsed`. The cost is averaged with the
/// previous one (equal weights), so one slow pass does not halve the next depth.
pub fn record_rerank_cost(candidates: usize, elapsed: Duration) {
    if candidates == 0 {
        return;
    }
    let cost = (elapsed.as_micros() as u64 / candidates as u64).max(1);
    let _ = COST_PER_CANDIDATE_US
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |prev| Some(averaged_cost(prev, cost)));
}

/// Next measurement from the previous one (`0` = none) and a new pass's cost.
fn averaged_cost(prev: u64, cost: u64) -> u64 {
    if prev == 0 {
        cost
    } else {
        (prev + cost) / 2
    }
}

/// Measured cost per candidate, if a pass ran in this process.
pub fn measured_cost_us() -> Option<u64> {
    match COST_PER_CANDIDATE_US.load(Ordering::Relaxed) {
        0 => None,
        v => Some(v),
    }
}

/// Depth a request should use now: `depth` within `budget_ms` at the measured cost, or `depth`
/// itself without a budget or a measurement.
pub fn effective_depth(depth: usize, budget_ms: Option<u64>) -> usize {
    match (budget_ms, measured_cost_us()) {
        (Some(budget), Some(cost)) => depth_within_budget(depth, budget, cost),
        _ => depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depths_follow_the_profile_table() {
        use ProfileTier::*;
        use SearchMode::*;
        let table = [
            (Lite, 20, 30),
            (SmartBalanced, 30, 50),
            (AirGappedPower, 30, 50),
            (WorkstationPro, 40, 75),
            (SovereignUltra, 50, 100),
        ];
        for (tier, fast, deep) in table {
            assert_eq!(profile_depth(tier, Fast), fast, "{tier:?} Fast");
            assert_eq!(profile_depth(tier, Deep), deep, "{tier:?} Deep");
        }
        assert_eq!(profile_depth(SmartBalanced, Fast), DEFAULT_RERANK_DEPTH);
    }

    #[test]
    fn the_budget_lowers_the_depth_to_fit() {
        // SC-004: 50 ms per candidate and a 1 s Fast budget leave 20.
        assert_eq!(depth_within_budget(30, time_budget_ms(SearchMode::Fast), 50_000), 20);
        assert_eq!(depth_within_budget(50, time_budget_ms(SearchMode::Deep), 50_000), 50);
    }

    #[test]
    fn the_budget_never_raises_the_depth_nor_goes_below_the_floor() {
        assert_eq!(depth_within_budget(30, 1_000, 1_000), 30);
        assert_eq!(depth_within_budget(30, 1_000, 1_000_000), MIN_RERANK_DEPTH);
        // A depth already under the floor stays as it is.
        assert_eq!(depth_within_budget(5, 1_000, 1_000_000), 5);
        assert_eq!(depth_within_budget(30, 1_000, 0), 30);
    }

    #[test]
    fn a_new_cost_is_averaged_with_the_previous_one() {
        assert_eq!(averaged_cost(0, 40_000), 40_000);
        assert_eq!(averaged_cost(40_000, 80_000), 60_000);
    }

    #[test]
    fn a_measured_cost_is_used_only_with_a_budget() {
        // Other tests may record passes concurrently, so only facts that hold for any cost.
        record_rerank_cost(10, Duration::from_millis(400));
        assert!(measured_cost_us().is_some());
        assert_eq!(effective_depth(30, None), 30);
        assert!(effective_depth(30, Some(1_000)) <= 30);
        assert!(effective_depth(30, Some(1_000)) >= MIN_RERANK_DEPTH);
    }
}
