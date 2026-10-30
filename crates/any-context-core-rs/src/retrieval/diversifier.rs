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

/// Source identifier used by the per-source cap: file path, else file name.
fn source_id(c: &RankedChunk) -> &str {
    if !c.file_path.is_empty() {
        &c.file_path
    } else if !c.file_name.is_empty() {
        &c.file_name
    } else {
        "unknown_source"
    }
}

/// Selects up to `target_k` chunks from a score-ordered list, at most `max_per_source` per source.
/// When fewer than `target_k` are selected, the skipped chunks fill the remaining slots, so the cap
/// gives other sources priority without leaving a slot empty. The selection keeps the input order;
/// `max_per_source = 0` means no cap (ADR-0139).
pub fn apply_source_cap(
    chunks: Vec<RankedChunk>,
    max_per_source: usize,
    target_k: usize,
) -> Vec<RankedChunk> {
    let mut per_source: HashMap<&str, usize> = HashMap::new();
    let mut keep = vec![false; chunks.len()];
    let mut selected = 0;

    for (i, c) in chunks.iter().enumerate() {
        if selected >= target_k {
            break;
        }
        let count = per_source.entry(source_id(c)).or_insert(0);
        if max_per_source == 0 || *count < max_per_source {
            *count += 1;
            keep[i] = true;
            selected += 1;
        }
    }

    for slot in keep.iter_mut() {
        if selected >= target_k {
            break;
        }
        if !*slot {
            *slot = true;
            selected += 1;
        }
    }

    chunks
        .into_iter()
        .zip(keep)
        .filter_map(|(c, k)| k.then_some(c))
        .collect()
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

    fn chunk(id: &str, score: f64, file: &str) -> RankedChunk {
        RankedChunk {
            id: id.into(),
            score,
            file_path: file.into(),
            file_name: file.into(),
            text: id.to_uppercase(),
            workspace: "W".into(),
            content_type: "PDF".into(),
        }
    }

    fn ids(chunks: &[RankedChunk]) -> Vec<&str> {
        chunks.iter().map(|c| c.id.as_str()).collect()
    }

    #[test]
    fn cap_keeps_score_order() {
        let chunks = vec![
            chunk("a1", 0.95, "A.pdf"),
            chunk("a2", 0.94, "A.pdf"),
            chunk("a3", 0.93, "A.pdf"),
            chunk("b1", 0.89, "B.pdf"),
            chunk("b2", 0.88, "B.pdf"),
            chunk("c1", 0.85, "C.pdf"),
        ];
        let capped = apply_source_cap(chunks, 2, 5);
        assert_eq!(ids(&capped), vec!["a1", "a2", "b1", "b2", "c1"]);
    }

    #[test]
    fn weak_other_source_does_not_displace_strong_second_chunk() {
        let chunks = vec![chunk("a1", 0.95, "A.pdf"), chunk("a2", 0.94, "A.pdf"), chunk("b1", 0.30, "B.pdf")];
        assert_eq!(ids(&apply_source_cap(chunks, 3, 2)), vec!["a1", "a2"]);
    }

    #[test]
    fn cap_limits_a_dominant_source_in_favor_of_others() {
        let chunks = vec![
            chunk("a1", 0.95, "A.pdf"),
            chunk("a2", 0.94, "A.pdf"),
            chunk("a3", 0.93, "A.pdf"),
            chunk("a4", 0.92, "A.pdf"),
            chunk("b1", 0.50, "B.pdf"),
        ];
        assert_eq!(ids(&apply_source_cap(chunks, 3, 4)), vec!["a1", "a2", "a3", "b1"]);
    }

    #[test]
    fn flexible_cap_backfills_in_score_order() {
        let single: Vec<RankedChunk> = (1..=6).map(|i| chunk(&format!("a{i}"), 1.0 - i as f64 * 0.01, "A.pdf")).collect();
        assert_eq!(ids(&apply_source_cap(single, 3, 6)), vec!["a1", "a2", "a3", "a4", "a5", "a6"]);

        // a3 exceeds the cap of 2 but is backfilled at its score position, ahead of b1.
        let mixed = vec![
            chunk("a1", 0.95, "A.pdf"),
            chunk("a2", 0.94, "A.pdf"),
            chunk("a3", 0.93, "A.pdf"),
            chunk("b1", 0.50, "B.pdf"),
        ];
        assert_eq!(ids(&apply_source_cap(mixed, 2, 4)), vec!["a1", "a2", "a3", "b1"]);
    }

    #[test]
    fn zero_cap_means_no_cap_and_empty_input_is_empty() {
        let chunks = vec![chunk("a1", 0.95, "A.pdf"), chunk("a2", 0.94, "A.pdf"), chunk("b1", 0.30, "B.pdf")];
        assert_eq!(ids(&apply_source_cap(chunks, 0, 2)), vec!["a1", "a2"]);
        assert!(apply_source_cap(Vec::new(), 3, 5).is_empty());
    }
}
