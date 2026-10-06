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

#[test]
fn test_sync_command_execution_and_log_redirection() {
    let ctx = ExecutionContext::default();

    // 1. Menu option flag or empty args opens sync menu
    let res_menu = CommandEngine::execute("sync", &["--menu"], &ctx);
    assert_eq!(res_menu.action, CommandAction::OpenMenu("sync".to_string()));

    let res_empty = CommandEngine::execute("sync", &[], &ctx);
    assert_eq!(res_empty.action, CommandAction::OpenMenu("sync".to_string()));

    // 2. --incremental flag executes incremental sync
    let res_inc = CommandEngine::execute("sync", &["--incremental"], &ctx);
    if res_inc.success {
        assert!(res_inc.message.contains("Synchronizing workspace"));
        assert!(res_inc.message.contains("Background synchronization worker spawned"));
    }

    // 3. Force flag executes forced full sync
    let res = CommandEngine::execute("sync", &["--force"], &ctx);
    if res.success {
        assert!(res.message.contains("sync_Default.log"));
        assert!(res.message.contains("Background synchronization worker spawned"));
    } else {
        assert!(res.message.contains("Failed to launch background synchronization worker"));
    }
}

#[test]
fn test_switch_command_options_and_deletion() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext {
        active_workspace: "TempDeletable".to_string(),
        ..Default::default()
    };

    // 1. Help flag returns clear options
    let res_help = CommandEngine::execute("switch", &["--help"], &ctx);
    assert!(res_help.success);
    assert!(res_help.message.contains("--delete"));
    assert!(res_help.message.contains("--list"));

    // 2. Reject deletion of protected Default workspace
    let res_del_default = CommandEngine::execute("switch", &["--delete", "Default"], &ctx);
    assert!(!res_del_default.success);
    assert!(res_del_default.message.contains("Cannot delete the protected 'Default' workspace"));

    // 3. Reject deletion of protected Global workspace
    let res_del_global = CommandEngine::execute("switch", &["delete", "Global"], &ctx);
    assert!(!res_del_global.success);
    assert!(res_del_global.message.contains("Cannot delete the protected 'Global' workspace"));
}

#[test]
fn test_folder_command_options_and_dispatch() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext {
        active_workspace: "FolderTestWS".to_string(),
        ..Default::default()
    };

    // 1. List folders when empty
    let res_list = CommandEngine::execute("folder", &[], &ctx);
    assert!(res_list.success);
    assert!(res_list.message.contains("Monitored Folders"));

    // 2. Add temporary folder
    let temp_dir = std::env::temp_dir().join("actx_folder_cmd_test");
    let _ = std::fs::create_dir_all(&temp_dir);
    let temp_str = temp_dir.to_string_lossy().to_string();

    let res_add = CommandEngine::execute("folder", &[&temp_str], &ctx);
    assert!(res_add.success);
    assert!(res_add.message.contains("Added folder to workspace"));

    // 3. Remove folder
    let res_remove = CommandEngine::execute("folder", &["--remove", &temp_str], &ctx);
    assert!(res_remove.success);
    assert!(res_remove.message.contains("Removed folder"));

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_web_command_options_and_dispatch() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext {
        active_workspace: "WebTestWS".to_string(),
        ..Default::default()
    };

    // 1. List web portals
    let res_list = CommandEngine::execute("web", &[], &ctx);
    assert!(res_list.success);
    assert!(res_list.message.contains("Web Documentation Portals"));

    // 2. Add web portal via direct URL
    let res_add = CommandEngine::execute("web", &["https://actx.dev/docs"], &ctx);
    assert!(res_add.success);
    assert!(res_add.message.contains("Added web documentation portal"));

    // 3. Remove web portal
    let res_remove = CommandEngine::execute("web", &["--remove", "https://actx.dev/docs"], &ctx);
    assert!(res_remove.success);
    assert!(res_remove.message.contains("Removed web source"));
}

#[test]
fn test_keys_command_options_and_persistence() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext::default();

    // 1. No args opens keys menu
    let res_menu = CommandEngine::execute("keys", &[], &ctx);
    assert_eq!(res_menu.action, CommandAction::OpenMenu("keys".to_string()));

    // 2. Audit shows provider audit
    let res_audit = CommandEngine::execute("keys", &["audit"], &ctx);
    assert!(res_audit.success);
    assert!(res_audit.message.contains("Provider Credentials Audit"));
    assert!(res_audit.message.contains("OpenAI"));

    // 3. Set API key persists and sets env var
    let res_set = CommandEngine::execute("keys", &["openai", "sk-test-anycontext-secret-key-123"], &ctx);
    assert!(res_set.success);
    assert_eq!(res_set.action, CommandAction::RebuildAgent);
    assert_eq!(std::env::var("OPENAI_API_KEY").ok(), Some("sk-test-anycontext-secret-key-123".to_string()));
}

#[test]
fn test_config_dashboard_and_get_set() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext {
        active_workspace: "ConfigTestWS".to_string(),
        ..Default::default()
    };

    // 1. No args returns full system dashboard
    let res_dash = CommandEngine::execute("config", &[], &ctx);
    assert!(res_dash.success);
    assert!(res_dash.message.contains("AnyContext System Configuration"));
    assert!(res_dash.message.contains("ConfigTestWS"));
    assert!(res_dash.message.contains("Grounding Strategy"));

    // 2. Set custom config
    let res_set = CommandEngine::execute("config", &["custom_timeout", "120s"], &ctx);
    assert!(res_set.success);
    assert!(res_set.message.contains("custom_timeout"));

    // 3. Get custom config
    let res_get = CommandEngine::execute("config", &["custom_timeout"], &ctx);
    assert!(res_get.success);
    assert!(res_get.message.contains("120s"));
}

#[test]
fn test_purge_command_execution() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext {
        active_workspace: "PurgeTestWS".to_string(),
        ..Default::default()
    };

    let res = CommandEngine::execute("purge", &[], &ctx);
    assert!(res.success);
    assert!(res.message.contains("Vector index and file hash metadata purged"));
}

#[test]
fn test_vision_command_toggle() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext::default();

    let res_on = CommandEngine::execute("vision", &["on"], &ctx);
    assert!(res_on.success);
    assert!(res_on.message.contains("ENABLED"));

    let res_off = CommandEngine::execute("vision", &["off"], &ctx);
    assert!(res_off.success);
    assert!(res_off.message.contains("DISABLED"));

    let res_status = CommandEngine::execute("vision", &[], &ctx);
    assert!(res_status.success);
    assert!(res_status.message.contains("DISABLED"));
}

#[test]
fn test_inspect_and_logs_execution() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext {
        active_workspace: "InspectTestWS".to_string(),
        ..Default::default()
    };

    let res_inspect = CommandEngine::execute("inspect", &[], &ctx);
    assert!(res_inspect.success);
    assert!(res_inspect.message.contains("Vector Store Inspection for `InspectTestWS`"));

    let res_logs = CommandEngine::execute("logs", &[], &ctx);
    assert!(res_logs.success);
}

