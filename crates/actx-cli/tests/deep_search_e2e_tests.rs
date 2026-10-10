//! End-to-End integration tests for Deep Search engine wiring with NativeHybridPipeline

use actx_cli::commands::registry::find_command;
use actx_cli::engine::build_agent_sync;
use actx_lm::providers::MockLmProvider;
use any_context_core_rs::commands::{CommandEngine, ExecutionContext};
use std::sync::Arc;

#[test]
fn test_slash_command_registry_includes_deep_search() {
    let fast_cmd = find_command("fast").expect("Command /fast should exist");
    assert_eq!(fast_cmd.name, "fast");
    assert_eq!(fast_cmd.category, "RAG");

    let deep_cmd = find_command("deep").expect("Command /deep should exist");
    assert_eq!(deep_cmd.name, "deep");
    assert_eq!(deep_cmd.category, "RAG");

    let search_cmd = find_command("search").expect("Command /search should exist");
    assert_eq!(search_cmd.name, "search");
    assert_eq!(search_cmd.category, "RAG");
}

#[test]
fn test_command_engine_search_depth_dispatch() {
    let ctx = ExecutionContext {
        active_workspace: "test_ws".to_string(),
        active_model: "gpt-4o".to_string(),
        grounding_mode: "hybrid".to_string(),
        search_mode: "auto".to_string(),
        web_search_enabled: false,
    };

    // 1. /search deep
    let res = CommandEngine::execute("search", &["deep"], &ctx);
    assert!(res.success);
    assert!(res.message.contains("DEEP"));
    assert_eq!(res.state_updates.search_mode, Some("deep".to_string()));

    // 2. /search fast
    let res = CommandEngine::execute("search", &["fast"], &ctx);
    assert!(res.success);
    assert!(res.message.contains("FAST"));
    assert_eq!(res.state_updates.search_mode, Some("fast".to_string()));

    // 3. /deep shortcut
    let res = CommandEngine::execute("deep", &["explain", "architecture"], &ctx);
    assert!(res.success);
    assert!(res.message.contains("Deep Search"));
    assert_eq!(res.state_updates.search_mode, Some("deep".to_string()));
}

#[tokio::test]
async fn test_e2e_build_agent_and_deep_search_run() {
    let provider = Arc::new(MockLmProvider::new());

    // Build agent with search_mode = "deep"
    let agent = build_agent_sync(
        provider,
        "mock-model",
        "Default",
        "hybrid",
        "deep",
        false,
    )
    .expect("Failed to build agent in deep mode");

    let response = agent
        .run("How does AnyContext architecture function?", None)
        .await
        .expect("Agent execution in deep mode failed");

    assert!(!response.content.is_empty());
    assert_eq!(response.total_turns, 1);
}
