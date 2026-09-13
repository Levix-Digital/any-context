use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RRFScoreResult {
    pub id: String,
    pub score: f64,
    pub dense_rank: Option<usize>,
    pub sparse_rank: Option<usize>,
}

/// Computes Reciprocal Rank Fusion (RRF) across ranked candidate lists.
///
/// Formula:
/// RRF_Score(d) = ∑ [ 1.0 / (k + rank_m(d)) ]
///
/// where rank_m(d) is the 1-based rank (1, 2, 3, ...) of document d in ranking list m.
/// If document d is absent from a ranking list, its term for that list is 0.
///
/// Default k is 60.0, per standard literature and industrial vector search engines.
pub fn reciprocal_rank_fusion(
    dense_ids: &[String],
    sparse_ids: &[String],
    k: Option<usize>,
) -> Vec<RRFScoreResult> {
    let k_val = k.unwrap_or(60) as f64;
    let mut scores: HashMap<String, (f64, Option<usize>, Option<usize>)> = HashMap::new();

    // 1. Process dense rankings
    for (idx, doc_id) in dense_ids.iter().enumerate() {
        let rank_1_based = idx + 1;
        let rrf_contrib = 1.0 / (k_val + rank_1_based as f64);
        let entry = scores.entry(doc_id.clone()).or_insert((0.0, None, None));
        entry.0 += rrf_contrib;
        entry.1 = Some(rank_1_based);
    }

    // 2. Process sparse (BM25) rankings
    for (idx, doc_id) in sparse_ids.iter().enumerate() {
        let rank_1_based = idx + 1;
        let rrf_contrib = 1.0 / (k_val + rank_1_based as f64);
        let entry = scores.entry(doc_id.clone()).or_insert((0.0, None, None));
        entry.0 += rrf_contrib;
        entry.2 = Some(rank_1_based);
    }

    // 3. Convert to result list and sort descending by score
    let mut results: Vec<RRFScoreResult> = scores
        .into_iter()
        .map(|(id, (score, dense_rank, sparse_rank))| RRFScoreResult {
            id,
            score,
            dense_rank,
            sparse_rank,
        })
        .collect();

    // Sort descending by RRF score; break ties with lower dense rank, then lower sparse rank
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let a_d = a.dense_rank.unwrap_or(usize::MAX);
                let b_d = b.dense_rank.unwrap_or(usize::MAX);
                a_d.cmp(&b_d)
            })
            .then_with(|| {
                let a_s = a.sparse_rank.unwrap_or(usize::MAX);
                let b_s = b.sparse_rank.unwrap_or(usize::MAX);
                a_s.cmp(&b_s)
            })
    });

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_both_top1() {
        let dense = vec!["doc_a".to_string(), "doc_b".to_string()];
        let sparse = vec!["doc_a".to_string(), "doc_c".to_string()];

        let fused = reciprocal_rank_fusion(&dense, &sparse, Some(60));
        assert_eq!(fused[0].id, "doc_a");
        // Score = 1/(60+1) + 1/(60+1) = 2/61 ≈ 0.03278
        let expected = (1.0 / 61.0) + (1.0 / 61.0);
        assert!((fused[0].score - expected).abs() < 1e-6);
        assert_eq!(fused[0].dense_rank, Some(1));
        assert_eq!(fused[0].sparse_rank, Some(1));
    }

    #[test]
    fn test_rrf_exact_code_bm25_boost() {
        // doc_exact was rank 1 in BM25, not in dense top-2
        // doc_sem was rank 1 in dense, not in BM25 top-2
        let dense = vec!["doc_sem".to_string(), "doc_other".to_string()];
        let sparse = vec!["doc_exact".to_string(), "doc_sem".to_string()];

        let fused = reciprocal_rank_fusion(&dense, &sparse, Some(60));
        // doc_sem is in both (dense rank 1, sparse rank 2) -> 1/61 + 1/62 ≈ 0.0325
        // doc_exact is rank 1 in sparse only -> 1/61 ≈ 0.01639
        assert_eq!(fused[0].id, "doc_sem");
        assert_eq!(fused[1].id, "doc_exact");
    }
}
