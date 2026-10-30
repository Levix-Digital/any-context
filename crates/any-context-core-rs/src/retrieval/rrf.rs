//! Reciprocal Rank Fusion (RRF) of any number of ranked lists (IMP-041).
//!
//! `RRF(d) = Σ 1 / (k + rank_m(d))` over every list `m` that contains `d` (1-based rank), with
//! equal weights and k = 60 by default. Each list belongs to one question of the search (the Fast
//! question, or one Deep sub-question), so a fused candidate also says which questions found it
//! and which one ranked it best.

use std::collections::HashMap;

/// Candidates of one retriever and one source for one question, best first.
#[derive(Debug, Clone, Default)]
pub struct RankedList {
    /// Index of the question (request) the list belongs to.
    pub query: usize,
    pub ids: Vec<String>,
}

/// One chunk after fusion.
#[derive(Debug, Clone, PartialEq)]
pub struct FusedCandidate {
    pub id: String,
    pub score: f64,
    /// Questions whose lists contain the chunk, in ascending order.
    pub queries: Vec<usize>,
    /// Question of the chunk's highest single contribution (lowest index on a tie).
    pub best_query: usize,
}

/// Fuses `lists` with RRF constant `k`. Order: score descending, then the best single
/// contribution, then the id, so equal inputs always fuse the same way.
pub fn fuse(lists: &[RankedList], k: usize) -> Vec<FusedCandidate> {
    let k = k as f64;
    // id -> (score, best contribution, best query, queries)
    let mut acc: HashMap<&str, (f64, f64, usize, Vec<usize>)> = HashMap::new();
    for list in lists {
        for (idx, id) in list.ids.iter().enumerate() {
            let contribution = 1.0 / (k + (idx + 1) as f64);
            let entry = acc.entry(id.as_str()).or_insert((0.0, 0.0, list.query, Vec::new()));
            entry.0 += contribution;
            if contribution > entry.1 || (contribution == entry.1 && list.query < entry.2) {
                entry.1 = contribution;
                entry.2 = list.query;
            }
            if !entry.3.contains(&list.query) {
                entry.3.push(list.query);
            }
        }
    }

    let mut fused: Vec<(FusedCandidate, f64)> = acc
        .into_iter()
        .map(|(id, (score, best, best_query, mut queries))| {
            queries.sort_unstable();
            (FusedCandidate { id: id.to_string(), score, queries, best_query }, best)
        })
        .collect();
    fused.sort_by(|(a, a_best), (b, b_best)| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b_best.partial_cmp(a_best).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.id.cmp(&b.id))
    });
    fused.into_iter().map(|(c, _)| c).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(query: usize, ids: &[&str]) -> RankedList {
        RankedList { query, ids: ids.iter().map(|s| s.to_string()).collect() }
    }

    #[test]
    fn a_chunk_first_in_two_lists_scores_both_contributions() {
        let fused = fuse(&[list(0, &["a", "b"]), list(0, &["a", "c"])], 60);
        assert_eq!(fused[0].id, "a");
        assert!((fused[0].score - 2.0 / 61.0).abs() < 1e-12);
        assert_eq!(fused.len(), 3);
    }

    #[test]
    fn a_chunk_in_both_lists_beats_a_first_place_in_one() {
        let fused = fuse(&[list(0, &["sem", "other"]), list(0, &["exact", "sem"])], 60);
        assert_eq!(fused[0].id, "sem");
        assert_eq!(fused[1].id, "exact");
    }

    #[test]
    fn every_list_counts_in_one_fusion() {
        let lists: Vec<RankedList> = (0..5).map(|q| list(q, &["x", "y"])).collect();
        let fused = fuse(&lists, 60);
        assert!((fused[0].score - 5.0 / 61.0).abs() < 1e-12);
        assert_eq!(fused[0].queries, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn the_best_query_is_the_highest_single_contribution() {
        // "c" is third for question 0 and first for question 1.
        let fused = fuse(&[list(0, &["a", "b", "c"]), list(1, &["c"])], 60);
        let c = fused.iter().find(|f| f.id == "c").unwrap();
        assert_eq!(c.queries, vec![0, 1]);
        assert_eq!(c.best_query, 1);
        // On a tie, the lower question index wins.
        let fused = fuse(&[list(1, &["d"]), list(0, &["d"])], 60);
        assert_eq!(fused[0].best_query, 0);
    }

    #[test]
    fn ties_are_ordered_by_id() {
        let fused = fuse(&[list(0, &["b"]), list(0, &["a"])], 60);
        assert_eq!(fused.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), ["a", "b"]);
    }

    #[test]
    fn no_lists_give_nothing() {
        assert!(fuse(&[], 60).is_empty());
    }
}
