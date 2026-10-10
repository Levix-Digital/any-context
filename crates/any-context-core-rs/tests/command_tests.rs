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

#[test]
fn test_sync_cancel_execution() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ctx = ExecutionContext {
        active_workspace: "CancelSyncWS".to_string(),
        ..Default::default()
    };

    // When no worker is active, reports idle/no worker gracefully
    let res_cancel = CommandEngine::execute("sync", &["cancel"], &ctx);
    assert!(res_cancel.success);
    assert!(res_cancel.message.contains("No active synchronization worker is currently running"));

    // Shortcut `/cancel` routes to sync cancel
    let res_shortcut = CommandEngine::execute("cancel", &[], &ctx);
    assert!(res_shortcut.success);
    assert!(res_shortcut.message.contains("No active synchronization worker is currently running"));
}

#[test]
fn test_cd_and_pwd_execution() {
    let ctx = ExecutionContext::default();

    // 1. /pwd reports current directory
    let res_pwd = CommandEngine::execute("pwd", &[], &ctx);
    assert!(res_pwd.success);
    assert!(res_pwd.message.contains("Current Working Directory"));

    // 2. /cd to existing dir
    let temp_dir = std::env::temp_dir();
    let temp_str = temp_dir.to_string_lossy().to_string();
    let res_cd = CommandEngine::execute("cd", &[&temp_str], &ctx);
    assert!(res_cd.success);
    assert!(res_cd.message.contains("Changed working directory to"));

    // 3. /cd to non-existent dir reports error
    let res_err = CommandEngine::execute("cd", &["this_directory_should_not_exist_xyz123"], &ctx);
    assert!(!res_err.success);
    assert!(res_err.message.contains("Directory does not exist"));
}

#[test]
fn test_inspect_transparent_decryption() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    std::env::set_var("ACTX_MACHINE_ID", "test_inspect_decrypt_node");

    let ws = "CryptoInspectWS";
    let ctx = ExecutionContext {
        active_workspace: ws.to_string(),
        ..Default::default()
    };

    let lance_path = any_context_core_rs::storage::get_default_lancedb_path();
    let lance = any_context_core_rs::storage::NativeLanceStore::open(&lance_path).expect("open lance");

    let sec = any_context_core_rs::security::NativeSecurityEngine::get_instance();
    let plaintext_secret = "Top Secret Architectural Source Code";
    let encrypted_payload = sec.encrypt_text(plaintext_secret);
    assert!(encrypted_payload.starts_with("enc::"));

    let rec = any_context_core_rs::storage::VectorRecord {
        id: "crypto_chunk_1".to_string(),
        vector: vec![0.0; 1536],
        text: encrypted_payload.clone(),
        file_name: "secret.rs".to_string(),
        file_path: "src/secret.rs".to_string(),
        workspace: ws.to_string(),
        last_modified: Some("2026-10-08T12:00:00Z".to_string()),
        content_type: Some("Rust Source Code".to_string()),
        document_summary: Some("Confidential module".to_string()),
        keywords: Some("secret, crypto".to_string()),
        content_hash: None,
    };

    lance.upsert_records(vec![rec], Some("workspace_chunks"), Some(1536)).expect("upsert failed in test_inspect_transparent_decryption");

    let res_full = CommandEngine::execute("inspect", &["--full"], &ctx);
    assert!(res_full.success);
    // Should NOT contain the raw encrypted ciphertext payload
    assert!(!res_full.message.contains(&encrypted_payload));
    // Should contain the decrypted plaintext
    assert!(res_full.message.contains(plaintext_secret));

    let _ = lance.delete_by_workspace(ws, None);
}

#[test]
fn test_update_command_syntax_and_audit_logging() {
    let ctx = ExecutionContext::default();

    // 1. /update@v99.99.99 syntax recognized by router and yields safe guidance
    let res1 = CommandEngine::execute("update@v99.99.99", &[], &ctx);
    assert!(!res1.message.contains("Unknown command"));
    assert!(res1.message.contains("actx --update@v99.99.99"));

    // 2. /update with @v99.99.99 argument recognized
    let res2 = CommandEngine::execute("update", &["@v99.99.99"], &ctx);
    assert!(!res2.message.contains("Unknown command"));
    assert!(res2.message.contains("actx --update@v99.99.99"));

    // 3. /update with --version=v99.99.99 argument recognized
    let res3 = CommandEngine::execute("update", &["--version=v99.99.99"], &ctx);
    assert!(!res3.message.contains("Unknown command"));
    assert!(res3.message.contains("actx --update@v99.99.99"));

    // 4. Verify update.log contains audit traces
    let logs_dir = actx_installer::paths::get_canonical_logs_dir();
    let update_log = logs_dir.join("update.log");
    assert!(update_log.exists(), "update.log should exist at {}", update_log.display());
    let log_content = std::fs::read_to_string(&update_log).unwrap_or_default();
    assert!(log_content.contains("v99.99.99"), "update.log should record requested target version tag");
}

#[test]
fn test_sources_command_rename_and_deprecation_of_shared() {
    std::env::set_var("ACTX_TEST_MODE", "1");
    let ws_name = format!("SourcesCmdWS_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
    let ctx = ExecutionContext {
        active_workspace: ws_name.clone(),
        ..Default::default()
    };

    // 1. Initial list empty
    let res_list = CommandEngine::execute("sources", &["active"], &ctx);
    assert!(res_list.success);
    assert!(res_list.message.contains(&format!("Data Sources configured for workspace '{}'", ws_name)));

    // 2. Add folder and web URL
    let _ = CommandEngine::execute("web", &["https://docs.rs/tokio/latest/tokio/index.html"], &ctx);
    let res_after_add = CommandEngine::execute("sources", &["active"], &ctx);
    assert!(res_after_add.success);
    assert!(res_after_add.message.contains("Tokio Docs"));

    // 3. Rename source via /sources rename
    let res_rename = CommandEngine::execute("sources", &["rename", "Tokio Docs", "Tokio Engine Core"], &ctx);
    assert!(res_rename.success);
    assert!(res_rename.message.contains("renamed to 'Tokio Engine Core'"));
    assert!(res_rename.message.contains("Chunks impacted in LanceDB: 0"));

    // 4. Verify display name updated
    let res_check = CommandEngine::execute("sources", &["active"], &ctx);
    assert!(res_check.message.contains("Tokio Engine Core"));

    // 5. Verify /shared deprecation error
    let res_shared = CommandEngine::execute("shared", &[], &ctx);
    assert!(!res_shared.success);
    assert!(res_shared.message.contains("deprecated in AnyContext v0.36.0"));
}


