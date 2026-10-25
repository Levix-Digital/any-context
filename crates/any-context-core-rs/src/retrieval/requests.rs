//! Retrieval requests the product sends, in one place (IMP-065).
//!
//! The chat's Fast search and each Deep Search sub-query use these builders, and so does the
//! development retrieval benchmark (`actx-bench`), so a change to a default here shows up in the
//! benchmark results.

use super::pipeline::{HybridSearchRequest, RRF_MIN_SCORE};

/// Workspace searched alongside the user's workspace for AnyContext's own documentation.
pub const GLOBAL_WORKSPACE: &str = "Global";

/// Request of a Fast (single-pass) search of `workspace` plus `Global`.
pub fn fast_request(query: &str, workspace: &str, rerank_mode: &str) -> HybridSearchRequest {
    HybridSearchRequest {
        query_text: query.to_string(),
        workspace: Some(workspace.to_string()),
        target_workspaces: vec![GLOBAL_WORKSPACE.to_string()],
        top_k: 6,
        candidate_pool_k: 30,
        max_density_chars: 14_000,
        min_score: RRF_MIN_SCORE,
        table_name: "workspace_chunks".to_string(),
        rerank_mode: Some(rerank_mode.to_string()),
        ..Default::default()
    }
}

/// Request sent for each Deep Search sub-query.
pub fn deep_request(query: &str, workspace: &str, rerank_mode: &str) -> HybridSearchRequest {
    HybridSearchRequest {
        query_text: query.to_string(),
        workspace: Some(workspace.to_string()),
        target_workspaces: vec![GLOBAL_WORKSPACE.to_string()],
        top_k: 8,
        candidate_pool_k: 40,
        max_density_chars: 16_000,
        min_score: RRF_MIN_SCORE,
        table_name: "workspace_chunks".to_string(),
        rerank_mode: Some(rerank_mode.to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_request_keeps_the_product_defaults() {
        let req = fast_request("q", "Docs", "auto");
        assert_eq!(req.query_text, "q");
        assert_eq!(req.workspace.as_deref(), Some("Docs"));
        assert_eq!(req.target_workspaces, vec!["Global".to_string()]);
        assert_eq!((req.top_k, req.candidate_pool_k, req.max_density_chars), (6, 30, 14_000));
        assert_eq!(req.min_score, RRF_MIN_SCORE);
        assert_eq!(req.table_name, "workspace_chunks");
        assert_eq!(req.rerank_mode.as_deref(), Some("auto"));
    }

    #[test]
    fn deep_request_keeps_the_product_defaults() {
        let req = deep_request("q", "Docs", "on");
        assert_eq!(req.target_workspaces, vec!["Global".to_string()]);
        assert_eq!((req.top_k, req.candidate_pool_k, req.max_density_chars), (8, 40, 16_000));
        assert_eq!(req.min_score, RRF_MIN_SCORE);
        assert_eq!(req.rerank_mode.as_deref(), Some("on"));
    }
}
