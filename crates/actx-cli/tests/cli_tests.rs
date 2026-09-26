use actx_cli::cli::args::{CliArgs, CliCommand};
use actx_cli::commands::registry::{autocomplete_commands, find_command};
use actx_cli::tui::app::{App, AppStatus};

#[test]
fn test_cli_args_headless_detection() {
    // 1. With command
    let args = CliArgs {
        workspace: "default".into(),
        prompt: None,
        model: None,
        command: Some(CliCommand::Diagnostics),
        positional_query: vec![],
    };
    assert!(args.is_headless());

    // 2. With prompt
    let args = CliArgs {
        workspace: "default".into(),
        prompt: Some("explain this project".into()),
        model: None,
        command: None,
        positional_query: vec![],
    };
    assert!(args.is_headless());

    // 3. With positional query
    let args = CliArgs {
        workspace: "default".into(),
        prompt: None,
        model: None,
        command: None,
        positional_query: vec!["how".into(), "does".into(), "it".into(), "work".into()],
    };
    assert!(args.is_headless());
}

#[test]
fn test_resolved_query() {
    let args = CliArgs {
        workspace: "default".into(),
        prompt: Some("query from flag".into()),
        model: None,
        command: None,
        positional_query: vec!["ignored".into()],
    };
    assert_eq!(args.resolved_query(), Some("query from flag".to_string()));

    let args = CliArgs {
        workspace: "default".into(),
        prompt: None,
        model: None,
        command: None,
        positional_query: vec!["positional".into(), "words".into()],
    };
    assert_eq!(args.resolved_query(), Some("positional words".to_string()));

    let args = CliArgs {
        workspace: "default".into(),
        prompt: None,
        model: None,
        command: None,
        positional_query: vec![],
    };
    assert_eq!(args.resolved_query(), None);
}

#[test]
fn test_slash_command_lookup_and_autocomplete() {
    // Exact lookup
    assert!(find_command("help").is_some());
    assert!(find_command("/help").is_some());
    assert!(find_command("workspace").is_some());
    assert!(find_command("nonexistent").is_none());

    // Autocomplete
    let matches = autocomplete_commands("/he");
    assert!(matches.iter().any(|c| c.name == "help"));

    let matches = autocomplete_commands("/s");
    assert!(matches.iter().any(|c| c.name == "sync"));
    assert!(matches.iter().any(|c| c.name == "status"));
    assert!(matches.iter().any(|c| c.name == "search"));
}

#[tokio::test]
async fn test_app_state_and_slash_dispatch() {
    let mut app = App::new("test-workspace".to_string(), "gpt-4o-mini".to_string(), None);
    assert_eq!(app.active_workspace, "test-workspace");
    assert_eq!(app.active_model, "gpt-4o-mini");
    assert_eq!(app.status, AppStatus::Idle);
    assert!(app.accordion_open);

    // Toggle accordion
    app.toggle_accordion();
    assert!(!app.accordion_open);
    app.toggle_accordion();
    assert!(app.accordion_open);

    // Typing and cursor
    app.insert_char('/');
    app.insert_char('w');
    assert!(app.slash_palette_open);
    assert!(!app.slash_matches.is_empty());

    // Test slash command execution: /workspace DevLab
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    app.input_buffer = "/workspace DevLab".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert_eq!(app.active_workspace, "DevLab");

    // Test /switch without args (lists workspaces)
    app.input_buffer = "/switch".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let last_msg = app.chat_history.last().expect("history item");
    assert!(last_msg.content.contains("Available Workspaces"));

    // Test /sync execution
    app.input_buffer = "/sync".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let sync_msg = app.chat_history.last().expect("sync item");
    assert!(sync_msg.content.contains("sync completed for workspace"));

    // Test /sync --force execution
    app.input_buffer = "/sync --force".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let force_sync_msg = app.chat_history.last().expect("force sync item");
    assert!(force_sync_msg.content.contains("Forced sync completed"));

    // Test /sources
    app.input_buffer = "/sources".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let sources_msg = app.chat_history.last().expect("sources item");
    assert!(sources_msg.content.contains("Workspace Sources"));

    // Test /diagnostics
    app.input_buffer = "/diagnostics".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let diag_msg = app.chat_history.last().expect("diag item");
    assert!(diag_msg.content.contains("AnyContext Diagnostics Report"));

    // Test /keys
    app.input_buffer = "/keys".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let keys_msg = app.chat_history.last().expect("keys item");
    assert!(keys_msg.content.contains("Provider Credentials Audit"));

    // Test /model claude-3-5
    app.input_buffer = "/model claude-3-5-sonnet".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert_eq!(app.active_model, "claude-3-5-sonnet");

    // Test /models (catalog)
    app.input_buffer = "/models".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let models_msg = app.chat_history.last().expect("models item");
    assert!(models_msg.content.contains("Supported AI Providers & Models"));

    // Test /menu
    app.input_buffer = "/menu".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert!(app.menu_state.is_open);
    app.close_menu();
    assert!(!app.menu_state.is_open);

    // Test /folder
    app.input_buffer = "/folder".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let folder_msg = app.chat_history.last().expect("folder item");
    assert!(folder_msg.content.contains("Monitored Folders"));

    // Test /web
    app.input_buffer = "/web".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let web_msg = app.chat_history.last().expect("web item");
    assert!(web_msg.content.contains("Web Documentation Portals"));

    // Test /mode deep
    app.input_buffer = "/mode deep".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let mode_msg = app.chat_history.last().expect("mode item");
    assert!(mode_msg.content.contains("Grounding Strategy Mode"));

    // Test /web-search on
    app.input_buffer = "/web-search on".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let ws_msg = app.chat_history.last().expect("ws item");
    assert!(ws_msg.content.contains("Real-time Web Search"));

    // Test /billing
    app.input_buffer = "/billing".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let billing_msg = app.chat_history.last().expect("billing item");
    assert!(billing_msg.content.contains("COMMUNITY"));

    // Test /reset-memory
    app.input_buffer = "/reset-memory".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let mem_msg = app.chat_history.last().expect("mem item");
    assert!(mem_msg.content.contains("Long-term session memory reset"));

    // Test /onboarding
    app.input_buffer = "/onboarding".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let onb_msg = app.chat_history.last().expect("onb item");
    assert!(onb_msg.content.contains("Welcome to AnyContext"));

    // Test /vision
    app.input_buffer = "/vision".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let vis_msg = app.chat_history.last().expect("vis item");
    assert!(vis_msg.content.contains("Vision LLM"));

    // Test /ocr
    app.input_buffer = "/ocr".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let ocr_msg = app.chat_history.last().expect("ocr item");
    assert!(ocr_msg.content.contains("OCR Engine Status"));

    // Test /clear
    app.input_buffer = "/clear".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert!(app.chat_history.is_empty());

    // Test /exit
    assert!(app.running);
    app.input_buffer = "/exit".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx);
    assert!(!app.running);
}
