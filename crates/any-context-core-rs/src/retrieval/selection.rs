//! Selection of the evidence of a search (IMP-045): the candidates, already in relevance order
//! (pinned matches, then the relevant band, then the maybe band), enter while their tokens fit the
//! request's budget. Near-identical texts enter once, a chunk that does not fit is skipped whole
//! (never truncated), and there is no per-file limit.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// Word 3-shingle Jaccard similarity at or above which two texts are the same evidence.
pub const NEAR_DUPLICATE_JACCARD: f64 = 0.9;

/// Words per shingle in the near-duplicate check.
const SHINGLE_WORDS: usize = 3;

/// What the selection of one search did, for the logs and the benchmark.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionCounts {
    /// Candidates that entered.
    pub selected: usize,
    /// Estimated tokens of the candidates that entered.
    pub tokens: usize,
    /// Candidates left out because they did not fit in what was left of the budget.
    pub over_budget: usize,
    /// Candidates left out as near-identical to one that entered.
    pub duplicates: usize,
}

/// One candidate as the selection sees it: its text and its estimated tokens.
pub struct Candidate<'a> {
    pub text: &'a str,
    pub tokens: usize,
}

/// Text compared for exact duplicates: lowercase, whitespace collapsed.
fn normalized(text: &str) -> String {
    text.split_whitespace().map(str::to_lowercase).collect::<Vec<_>>().join(" ")
}

/// Word shingles of a normalized text; empty when it has fewer words than one shingle.
fn shingles(normalized: &str) -> HashSet<&str> {
    let bounds: Vec<usize> = std::iter::once(0)
        .chain(normalized.match_indices(' ').map(|(i, _)| i + 1))
        .collect();
    let words = bounds.len();
    if normalized.is_empty() || words < SHINGLE_WORDS {
        return HashSet::new();
    }
    (0..=words - SHINGLE_WORDS)
        .map(|i| {
            let end = bounds.get(i + SHINGLE_WORDS).map(|&b| b - 1).unwrap_or(normalized.len());
            &normalized[bounds[i]..end]
        })
        .collect()
}

fn jaccard(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let shared = a.intersection(b).count();
    shared as f64 / (a.len() + b.len() - shared) as f64
}

/// Indices of `candidates` (in relevance order) that enter a context of `token_budget` tokens,
/// in that order, with what the selection did. The first non-duplicate candidate always enters,
/// even alone over the budget, so a relevant answer is never lost to it. Empty texts never enter.
pub fn select(candidates: &[Candidate<'_>], token_budget: usize) -> (Vec<usize>, SelectionCounts) {
    let norms: Vec<String> = candidates.iter().map(|c| normalized(c.text)).collect();
    let mut counts = SelectionCounts::default();
    let mut picked: Vec<usize> = Vec::new();
    let mut picked_norms: HashSet<&str> = HashSet::new();
    let mut picked_shingles: Vec<HashSet<&str>> = Vec::new();

    for (i, c) in candidates.iter().enumerate() {
        let norm = norms[i].as_str();
        if norm.is_empty() {
            continue;
        }
        let own = shingles(norm);
        if picked_norms.contains(norm) || picked_shingles.iter().any(|s| jaccard(&own, s) >= NEAR_DUPLICATE_JACCARD) {
            counts.duplicates += 1;
            continue;
        }
        if !picked.is_empty() && counts.tokens + c.tokens > token_budget {
            counts.over_budget += 1;
            continue;
        }
        counts.tokens += c.tokens;
        counts.selected += 1;
        picked.push(i);
        picked_norms.insert(norm);
        picked_shingles.push(own);
    }
    (picked, counts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(n: usize, tag: &str) -> String {
        (0..n).map(|i| format!("{tag}{i}")).collect::<Vec<_>>().join(" ")
    }

    fn run(items: &[(String, usize)], budget: usize) -> (Vec<usize>, SelectionCounts) {
        let candidates: Vec<Candidate<'_>> = items.iter().map(|(t, n)| Candidate { text: t, tokens: *n }).collect();
        select(&candidates, budget)
    }

    #[test]
    fn every_relevant_chunk_that_fits_enters_past_any_old_count() {
        let items: Vec<(String, usize)> = (0..9).map(|i| (words(40, &format!("r{i}x")), 300)).collect();
        let (picked, counts) = run(&items, 4_000);
        assert_eq!(picked, (0..9).collect::<Vec<_>>());
        assert_eq!((counts.selected, counts.tokens, counts.over_budget), (9, 2_700, 0));
    }

    #[test]
    fn relevant_then_the_best_maybe_up_to_the_budget() {
        let items: Vec<(String, usize)> = (0..12).map(|i| (words(40, &format!("c{i}x")), 500)).collect();
        let (picked, counts) = run(&items, 4_000);
        assert_eq!(picked, (0..8).collect::<Vec<_>>());
        assert_eq!((counts.tokens, counts.over_budget), (4_000, 4));
    }

    #[test]
    fn a_chunk_that_does_not_fit_is_skipped_whole_and_a_smaller_one_enters() {
        let items = vec![(words(20, "a"), 3_000), (words(20, "b"), 1_500), (words(20, "c"), 800)];
        let (picked, counts) = run(&items, 4_000);
        assert_eq!(picked, vec![0, 2]);
        assert_eq!((counts.tokens, counts.over_budget), (3_800, 1));
    }

    #[test]
    fn an_oversized_first_chunk_enters_alone() {
        let items = vec![(words(20, "a"), 9_000), (words(20, "b"), 100)];
        let (picked, counts) = run(&items, 4_000);
        assert_eq!(picked, vec![0]);
        assert_eq!((counts.selected, counts.tokens, counts.over_budget), (1, 9_000, 1));
    }

    #[test]
    fn equal_text_after_case_and_spacing_enters_once() {
        let items = vec![
            ("The Port is 8080\n\nfor the API".to_string(), 10),
            ("the port   is 8080 for THE api".to_string(), 10),
            ("Short".to_string(), 1),
            ("short".to_string(), 1),
        ];
        let (picked, counts) = run(&items, 4_000);
        assert_eq!(picked, vec![0, 2]);
        assert_eq!(counts.duplicates, 2);
    }

    #[test]
    fn near_identical_text_enters_once_and_different_text_stays() {
        let base = words(100, "w");
        let near = format!("{base} extra");
        let topic = format!("{} {}", words(50, "w"), words(50, "other"));
        let items = vec![(base, 100), (near, 100), (topic, 100)];
        let (picked, counts) = run(&items, 4_000);
        assert_eq!(picked, vec![0, 2]);
        assert_eq!(counts.duplicates, 1);
    }

    #[test]
    fn empty_input_empty_text_and_zero_budget() {
        assert_eq!(run(&[], 4_000).0, Vec::<usize>::new());
        let items = vec![("   ".to_string(), 0), (words(5, "a"), 50), (words(5, "b"), 50)];
        let (picked, counts) = run(&items, 0);
        assert_eq!(picked, vec![1]);
        assert_eq!((counts.selected, counts.over_budget), (1, 1));
    }

    #[test]
    fn shingles_cover_every_window_of_three_words() {
        let s = shingles("a b c d");
        assert_eq!(s, ["a b c", "b c d"].into_iter().collect::<HashSet<&str>>());
        assert!(shingles("a b").is_empty());
    }
}
