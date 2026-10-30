//! Retrieval requests the product sends, in one place (IMP-065).
//!
//! The chat's Fast search and each Deep Search sub-query use these builders, and so does the
//! development retrieval benchmark (`actx-bench`), so a change to a default here shows up in the
//! benchmark results.

use super::pipeline::HybridSearchRequest;
use super::rerank_depth::{profile_depth, time_budget_ms, SearchMode};
use crate::hardware::ProfileTier;

/// Candidates each retriever (dense and lexical) returns for one question or sub-question, in
/// Fast and Deep alike (IMP-040). Tuned by the benchmark: it goes up only if more candidates
/// improve the hit rate measurably.
pub const CANDIDATES_PER_LIST: usize = 30;

/// Workspace searched alongside the user's workspace for AnyContext's own documentation.
pub const GLOBAL_WORKSPACE: &str = "Global";

/// Request of a Fast (single-pass) search of `workspace` plus `Global` on hardware `tier`.
pub fn fast_request(query: &str, workspace: &str, rerank_mode: &str, tier: ProfileTier) -> HybridSearchRequest {
    HybridSearchRequest {
        query_text: query.to_string(),
        workspace: Some(workspace.to_string()),
        target_workspaces: vec![GLOBAL_WORKSPACE.to_string()],
        top_k: 6,
        candidate_pool_k: CANDIDATES_PER_LIST,
        max_density_chars: 14_000,
        table_name: "workspace_chunks".to_string(),
        rerank_mode: Some(rerank_mode.to_string()),
        rerank_depth: profile_depth(tier, SearchMode::Fast),
        rerank_budget_ms: Some(time_budget_ms(SearchMode::Fast)),
        ..Default::default()
    }
}

/// Request sent for each Deep Search sub-query on hardware `tier`. The batch fuses all
/// sub-queries and reranks them once, with the first request's depth (IMP-041).
pub fn deep_request(query: &str, workspace: &str, rerank_mode: &str, tier: ProfileTier) -> HybridSearchRequest {
    HybridSearchRequest {
        query_text: query.to_string(),
        workspace: Some(workspace.to_string()),
        target_workspaces: vec![GLOBAL_WORKSPACE.to_string()],
        top_k: 8,
        candidate_pool_k: CANDIDATES_PER_LIST,
        max_density_chars: 16_000,
        table_name: "workspace_chunks".to_string(),
        rerank_mode: Some(rerank_mode.to_string()),
        rerank_depth: profile_depth(tier, SearchMode::Deep),
        rerank_budget_ms: Some(time_budget_ms(SearchMode::Deep)),
        ..Default::default()
    }
}

/// Request of the agent's search tool: a Fast search with a smaller result.
pub fn tool_request(query: &str, workspace: &str, rerank_mode: &str, tier: ProfileTier) -> HybridSearchRequest {
    HybridSearchRequest { top_k: 5, max_density_chars: 12_000, ..fast_request(query, workspace, rerank_mode, tier) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_request_keeps_the_product_defaults() {
        let req = fast_request("q", "Docs", "auto", ProfileTier::SmartBalanced);
        assert_eq!(req.query_text, "q");
        assert_eq!(req.workspace.as_deref(), Some("Docs"));
        assert_eq!(req.target_workspaces, vec!["Global".to_string()]);
        assert_eq!((req.top_k, req.candidate_pool_k, req.max_density_chars), (6, 30, 14_000));
        assert_eq!(req.table_name, "workspace_chunks");
        assert_eq!(req.rerank_mode.as_deref(), Some("auto"));
        assert_eq!((req.rerank_depth, req.rerank_budget_ms), (30, Some(1_000)));
    }

    #[test]
    fn deep_request_keeps_the_product_defaults() {
        let req = deep_request("q", "Docs", "on", ProfileTier::SmartBalanced);
        assert_eq!(req.target_workspaces, vec!["Global".to_string()]);
        assert_eq!((req.top_k, req.candidate_pool_k, req.max_density_chars), (8, 30, 16_000));
        assert_eq!(req.rerank_mode.as_deref(), Some("on"));
        assert_eq!((req.rerank_depth, req.rerank_budget_ms), (50, Some(3_000)));
    }

    #[test]
    fn the_depth_follows_the_profile() {
        assert_eq!(fast_request("q", "D", "auto", ProfileTier::Lite).rerank_depth, 20);
        assert_eq!(deep_request("q", "D", "auto", ProfileTier::SovereignUltra).rerank_depth, 100);
    }

    #[test]
    fn tool_request_is_a_smaller_fast_request() {
        let req = tool_request("q", "Docs", "auto", ProfileTier::WorkstationPro);
        assert_eq!((req.top_k, req.max_density_chars), (5, 12_000));
        assert_eq!(req.target_workspaces, vec!["Global".to_string()]);
        assert_eq!((req.rerank_depth, req.rerank_budget_ms), (40, Some(1_000)));
    }
}
