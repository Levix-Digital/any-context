use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedChunk {
    pub id: String,
    pub score: f64,
    pub file_path: String,
    pub file_name: String,
    pub text: String,
    pub workspace: String,
    pub content_type: String,
}

/// Discards candidates falling below min_score threshold.
/// Retains at least the single best candidate if non-empty to prevent complete blackout.
pub fn apply_threshold(chunks: Vec<RankedChunk>, min_score: f64) -> Vec<RankedChunk> {
    if chunks.is_empty() {
        return Vec::new();
    }
    let filtered: Vec<RankedChunk> = chunks.into_iter().filter(|c| c.score >= min_score).collect();
    filtered
}

/// Applies Source-Fair Round-Robin allocation to guarantee multi-source representation.
/// Ensures that when a workspace contains 20+ websites or documents, all matching sources
/// are fairly represented in the context window without a single document monopolizing all slots.
pub fn apply_source_diversification(
    chunks: Vec<RankedChunk>,
    max_per_source: usize,
    target_k: usize,
) -> Vec<RankedChunk> {
    if chunks.is_empty() {
        return Vec::new();
    }

    // Group chunks by unique source identifier (file_path or file_name)
    let mut source_groups: HashMap<String, Vec<RankedChunk>> = HashMap::new();
    let mut source_order: Vec<String> = Vec::new();

    for c in chunks.clone() {
        let src_id = if !c.file_path.is_empty() {
            c.file_path.clone()
        } else if !c.file_name.is_empty() {
            c.file_name.clone()
        } else {
            "unknown_source".to_string()
        };

        if !source_groups.contains_key(&src_id) {
            source_order.push(src_id.clone());
        }
        source_groups.entry(src_id).or_default().push(c);
    }

    let mut selected: Vec<RankedChunk> = Vec::with_capacity(target_k);
    let mut selected_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Pass 1: Round-robin across distinct sources up to max_per_source
    for pass_idx in 0..max_per_source {
        for src_id in &source_order {
            if selected.len() >= target_k {
                break;
            }
            if let Some(group) = source_groups.get(src_id) {
                if pass_idx < group.len() {
                    let candidate = &group[pass_idx];
                    if !selected_ids.contains(&candidate.id) {
                        selected.push(candidate.clone());
                        selected_ids.insert(candidate.id.clone());
                    }
                }
            }
        }
        if selected.len() >= target_k {
            break;
        }
    }

    // Pass 2: If quota remains unfilled, backfill with remaining highest-scoring candidates
    if selected.len() < target_k {
        for c in chunks {
            if !selected_ids.contains(&c.id) {
                selected.push(c.clone());
                selected_ids.insert(c.id);
                if selected.len() >= target_k {
                    break;
                }
            }
        }
    }

    selected
}

/// Enforces a strict density budget in characters, condensing trailing chunks if needed.
pub fn apply_density_budget(chunks: Vec<RankedChunk>, max_chars: usize) -> Vec<RankedChunk> {
    if chunks.is_empty() {
        return Vec::new();
    }

    let mut budgeted = Vec::new();
    let mut accumulated_chars = 0;

    for (i, mut c) in chunks.into_iter().enumerate() {
        let chunk_len = c.text.len();
        if accumulated_chars + chunk_len > max_chars && i >= 3 {
            let remaining_space = max_chars.saturating_sub(accumulated_chars);
            if remaining_space > 200 {
                let mut truncated = c.text.chars().take(remaining_space).collect::<String>();
                truncated.push_str("\n[...additional snippet condensed for density limit...]");
                c.text = truncated;
                budgeted.push(c);
            }
            break;
        }

        accumulated_chars += chunk_len;
        budgeted.push(c);
    }

    budgeted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diversification_round_robin() {
        let chunks = vec![
            RankedChunk { id: "a1".into(), score: 0.95, file_path: "A.pdf".into(), file_name: "A.pdf".into(), text: "A1".into(), workspace: "W".into(), content_type: "PDF".into() },
            RankedChunk { id: "a2".into(), score: 0.94, file_path: "A.pdf".into(), file_name: "A.pdf".into(), text: "A2".into(), workspace: "W".into(), content_type: "PDF".into() },
            RankedChunk { id: "a3".into(), score: 0.93, file_path: "A.pdf".into(), file_name: "A.pdf".into(), text: "A3".into(), workspace: "W".into(), content_type: "PDF".into() },
            RankedChunk { id: "b1".into(), score: 0.89, file_path: "B.pdf".into(), file_name: "B.pdf".into(), text: "B1".into(), workspace: "W".into(), content_type: "PDF".into() },
            RankedChunk { id: "b2".into(), score: 0.88, file_path: "B.pdf".into(), file_name: "B.pdf".into(), text: "B2".into(), workspace: "W".into(), content_type: "PDF".into() },
            RankedChunk { id: "c1".into(), score: 0.85, file_path: "C.pdf".into(), file_name: "C.pdf".into(), text: "C1".into(), workspace: "W".into(), content_type: "PDF".into() },
        ];

        let diversified = apply_source_diversification(chunks, 2, 5);
        assert_eq!(diversified.len(), 5);
        let ids: Vec<String> = diversified.iter().map(|c| c.id.clone()).collect();
        // Should pick A1, B1, C1 (pass 1), then A2, B2 (pass 2)
        assert_eq!(ids, vec!["a1", "b1", "c1", "a2", "b2"]);
    }
}
