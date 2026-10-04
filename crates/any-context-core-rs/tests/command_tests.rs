//! Unit test suite verifying UI-agnostic execution in CommandEngine.

use any_context_core_rs::commands::{
    CommandAction, CommandEngine, ExecutionContext, GroundingMode, SearchDepthMode,
};

#[test]
fn test_grounding_mode_execution_and_isolation() {
    let ctx = ExecutionContext {
        active_workspace: "TestWS".to_string(),
        grounding_mode: "hybrid".to_string(),
        ..Default::default()
    };

    // Valid modes: strict, hybrid, proactive
    let res_strict = CommandEngine::execute("mode", &["strict"], &ctx);
    assert!(res_strict.success);
    assert_eq!(res_strict.state_updates.grounding_mode.as_deref(), Some("strict"));
    assert_eq!(res_strict.action, CommandAction::RebuildAgent);
    assert!(res_strict.message.contains("STRICT"));

    let res_hybrid = CommandEngine::execute("mode", &["hybrid"], &ctx);
    assert!(res_hybrid.success);
    assert_eq!(res_hybrid.state_updates.grounding_mode.as_deref(), Some("hybrid"));

    let res_proactive = CommandEngine::execute("mode", &["proactive"], &ctx);
    assert!(res_proactive.success);
    assert_eq!(res_proactive.state_updates.grounding_mode.as_deref(), Some("proactive"));

    // Invalid modes: auto, fast, deep are rejected from Grounding
    let res_invalid = CommandEngine::execute("mode", &["fast"], &ctx);
    assert!(!res_invalid.success);
    assert!(res_invalid.message.contains("Invalid grounding mode 'fast'"));
    assert!(res_invalid.message.contains("strict"));
}

#[test]
fn test_search_mode_execution_and_isolation() {
    let ctx = ExecutionContext {
        active_workspace: "TestWS".to_string(),
        search_mode: "auto".to_string(),
        ..Default::default()
    };

    // Valid modes: auto, fast, deep
    let res_auto = CommandEngine::execute("search", &["auto"], &ctx);
    assert!(res_auto.success);
    assert_eq!(res_auto.state_updates.search_mode.as_deref(), Some("auto"));
    assert_eq!(res_auto.action, CommandAction::RebuildAgent);
    assert!(res_auto.message.contains("AUTO"));

    let res_fast = CommandEngine::execute("search", &["fast"], &ctx);
    assert!(res_fast.success);
    assert_eq!(res_fast.state_updates.search_mode.as_deref(), Some("fast"));

    let res_deep = CommandEngine::execute("search", &["deep"], &ctx);
    assert!(res_deep.success);
    assert_eq!(res_deep.state_updates.search_mode.as_deref(), Some("deep"));

    // Invalid mode rejected from search
    let res_invalid = CommandEngine::execute("search", &["strict"], &ctx);
    assert!(!res_invalid.success);
    assert!(res_invalid.message.contains("Invalid search mode 'strict'"));
}

#[test]
fn test_fast_and_deep_shortcuts() {
    let ctx = ExecutionContext::default();

    let res_fast = CommandEngine::execute("fast", &[], &ctx);
    assert!(res_fast.success);
    assert_eq!(res_fast.state_updates.search_mode.as_deref(), Some("fast"));

    let res_deep = CommandEngine::execute("deep", &[], &ctx);
    assert!(res_deep.success);
    assert_eq!(res_deep.state_updates.search_mode.as_deref(), Some("deep"));
}

#[test]
fn test_web_search_toggle() {
    let ctx = ExecutionContext::default();

    let res_on = CommandEngine::execute("web-search", &["on"], &ctx);
    assert!(res_on.success);
    assert_eq!(res_on.state_updates.web_search_enabled, Some(true));
    assert_eq!(res_on.action, CommandAction::RebuildAgent);

    let res_off = CommandEngine::execute("web-search", &["off"], &ctx);
    assert!(res_off.success);
    assert_eq!(res_off.state_updates.web_search_enabled, Some(false));
}

#[test]
fn test_ui_agnostic_actions() {
    let ctx = ExecutionContext::default();

    let res_exit = CommandEngine::execute("exit", &[], &ctx);
    assert_eq!(res_exit.action, CommandAction::Exit);

    let res_clear = CommandEngine::execute("clear", &[], &ctx);
    assert_eq!(res_clear.action, CommandAction::ClearChat);

    let res_menu = CommandEngine::execute("menu", &[], &ctx);
    assert_eq!(res_menu.action, CommandAction::OpenMenu("main".to_string()));

    let res_reset = CommandEngine::execute("reset-memory", &[], &ctx);
    assert_eq!(res_reset.action, CommandAction::ClearChat);
    assert!(res_reset.message.contains("session memory reset"));
}

#[test]
fn test_grounding_and_search_mode_parsing() {
    assert_eq!(GroundingMode::parse("strict"), Some(GroundingMode::Strict));
    assert_eq!(GroundingMode::parse("s"), Some(GroundingMode::Strict));
    assert_eq!(GroundingMode::parse("hybrid"), Some(GroundingMode::Hybrid));
    assert_eq!(GroundingMode::parse("h"), Some(GroundingMode::Hybrid));
    assert_eq!(GroundingMode::parse("proactive"), Some(GroundingMode::Proactive));
    assert_eq!(GroundingMode::parse("p"), Some(GroundingMode::Proactive));
    assert_eq!(GroundingMode::parse("fast"), None);
    assert_eq!(GroundingMode::parse("deep"), None);
    assert_eq!(GroundingMode::parse("auto"), None);

    assert_eq!(SearchDepthMode::parse("auto"), Some(SearchDepthMode::Auto));
    assert_eq!(SearchDepthMode::parse("a"), Some(SearchDepthMode::Auto));
    assert_eq!(SearchDepthMode::parse("fast"), Some(SearchDepthMode::Fast));
    assert_eq!(SearchDepthMode::parse("f"), Some(SearchDepthMode::Fast));
    assert_eq!(SearchDepthMode::parse("deep"), Some(SearchDepthMode::Deep));
    assert_eq!(SearchDepthMode::parse("d"), Some(SearchDepthMode::Deep));
    assert_eq!(SearchDepthMode::parse("strict"), None);
    assert_eq!(SearchDepthMode::parse("hybrid"), None);
}
