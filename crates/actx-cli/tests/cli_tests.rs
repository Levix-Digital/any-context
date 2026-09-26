use actx_cli::cli::args::{CliArgs, CliCommand};
use actx_cli::commands::registry::{autocomplete_commands, find_command};
use actx_cli::tui::app::{App, AppStatus};

#[test]
fn test_cli_args_headless_detection() {
    // 1. With command
    let args = CliArgs {
        workspace: "default".into(),
        command: Some(CliCommand::Diagnostics),
        ..Default::default()
    };
    assert!(args.is_headless());

    // 2. With prompt
    let args = CliArgs {
        workspace: "default".into(),
        prompt: Some("explain this project".into()),
        ..Default::default()
    };
    assert!(args.is_headless());

    // 3. With positional query
    let args = CliArgs {
        workspace: "default".into(),
        positional_query: vec!["how".into(), "does".into(), "it".into(), "work".into()],
        ..Default::default()
    };
    assert!(args.is_headless());

    // 4. With top-level flags
    let args = CliArgs {
        check_update: true,
        ..Default::default()
    };
    assert!(args.is_headless());

    let args = CliArgs {
        diagnostics: true,
        ..Default::default()
    };
    assert!(args.is_headless());

    let args = CliArgs {
        sync: true,
        ..Default::default()
    };
    assert!(args.is_headless());
}

#[test]
fn test_resolved_query() {
    let args = CliArgs {
        workspace: "default".into(),
        prompt: Some("query from flag".into()),
        positional_query: vec!["ignored".into()],
        ..Default::default()
    };
    assert_eq!(args.resolved_query(), Some("query from flag".to_string()));

    let args = CliArgs {
        workspace: "default".into(),
        positional_query: vec!["positional".into(), "words".into()],
        ..Default::default()
    };
    assert_eq!(args.resolved_query(), Some("positional words".to_string()));

    let args = CliArgs {
        workspace: "default".into(),
        ..Default::default()
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

    // Test /switch without args (opens workspaces interactive modal)
    app.input_buffer = "/switch".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert!(app.menu_state.is_open);
    assert_eq!(app.menu_state.current_menu_id, "workspaces");
    app.close_menu();

    // Test /switch --list (lists workspaces directly)
    app.input_buffer = "/switch --list".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let last_msg = app.chat_history.last().expect("history item");
    assert!(last_msg.content.contains("Available Workspaces"));

    // Test /sync without args (opens sync interactive modal)
    app.input_buffer = "/sync".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert!(app.menu_state.is_open);
    assert_eq!(app.menu_state.current_menu_id, "sync");
    app.close_menu();

    // Test /sync --force execution
    app.input_buffer = "/sync --force".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let force_sync_msg = app.chat_history.last().expect("force sync item");
    assert!(force_sync_msg.content.contains("Forced sync completed"));

    // Test /sources without args (opens sources interactive modal)
    app.input_buffer = "/sources".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert!(app.menu_state.is_open);
    assert_eq!(app.menu_state.current_menu_id, "sources");
    app.close_menu();

    // Test /sources --all (lists all sources across workspaces)
    app.input_buffer = "/sources --all".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let sources_all_msg = app.chat_history.last().expect("sources all item");
    assert!(sources_all_msg.content.contains("All Configured Workspaces & Sources"));

    // Test /diagnostics
    app.input_buffer = "/diagnostics".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let diag_msg = app.chat_history.last().expect("diag item");
    assert!(diag_msg.content.contains("AnyContext Diagnostics Report"));

    // Test /keys without args (opens keys interactive modal)
    app.input_buffer = "/keys".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert!(app.menu_state.is_open);
    assert_eq!(app.menu_state.current_menu_id, "keys");
    app.close_menu();

    // Test /keys audit (direct credential audit)
    app.input_buffer = "/keys audit".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let keys_msg = app.chat_history.last().expect("keys item");
    assert!(keys_msg.content.contains("Provider Credentials Audit"));

    // Test /model claude-3-5
    app.input_buffer = "/model claude-3-5-sonnet".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert_eq!(app.active_model, "claude-3-5-sonnet");

    // Test /models without args (opens models interactive modal)
    app.input_buffer = "/models".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    assert!(app.menu_state.is_open);
    assert_eq!(app.menu_state.current_menu_id, "models");
    app.close_menu();

    // Test /models --list (catalog display)
    app.input_buffer = "/models --list".to_string();
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

#[test]
fn test_utf8_input_handling() {
    let mut app = App::new("test-workspace".to_string(), "gpt-4o-mini".to_string(), None);

    // 1. Type accented characters: "Qual é"
    // 'é' is 2 bytes (0xC3 0xA9)
    for c in "Qual é".chars() {
        app.insert_char(c);
    }
    assert_eq!(app.input_buffer, "Qual é");
    assert_eq!(app.cursor_idx, "Qual é".len()); // 7 bytes (5 ascii + 2 bytes for 'é')
    assert!(app.input_buffer.is_char_boundary(app.cursor_idx));

    // 2. Backspace deletes the multi-byte character cleanly without panicking
    app.delete_backspace();
    assert_eq!(app.input_buffer, "Qual ");
    assert_eq!(app.cursor_idx, 5);
    assert!(app.input_buffer.is_char_boundary(app.cursor_idx));

    // 3. Re-insert 'é' and more accents: "é ação"
    for c in "é ação".chars() {
        app.insert_char(c);
    }
    assert_eq!(app.input_buffer, "Qual é ação");
    assert_eq!(app.cursor_idx, "Qual é ação".len());
    assert!(app.input_buffer.is_char_boundary(app.cursor_idx));

    // 4. Cursor movement left across multi-byte characters
    // "Qual é ação" ends with 'o', then 'ã' (2 bytes), 'c' with cedilla 'ç' (2 bytes)
    app.move_cursor_left(); // before 'o'
    app.move_cursor_left(); // before 'ã'
    assert!(app.input_buffer.is_char_boundary(app.cursor_idx));

    // 5. Delete forward on multi-byte char 'ã'
    app.delete_forward();
    assert_eq!(app.input_buffer, "Qual é aço");
    assert!(app.input_buffer.is_char_boundary(app.cursor_idx));

    // 6. Test 4-byte UTF-8 emoji
    app.insert_char('🚀');
    assert!(app.input_buffer.contains('🚀'));
    assert!(app.input_buffer.is_char_boundary(app.cursor_idx));

    app.delete_backspace();
    assert!(!app.input_buffer.contains('🚀'));
    assert_eq!(app.input_buffer, "Qual é aço");
    assert!(app.input_buffer.is_char_boundary(app.cursor_idx));

    // 7. Move all the way to beginning and right across characters
    for _ in 0..20 {
        app.move_cursor_left();
    }
    assert_eq!(app.cursor_idx, 0);

    for _ in 0..app.input_buffer.chars().count() {
        app.move_cursor_right();
        assert!(app.input_buffer.is_char_boundary(app.cursor_idx));
    }
    assert_eq!(app.cursor_idx, app.input_buffer.len());
}

#[test]
fn test_paragraph_line_count_and_autoscroll() {
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

    let block = Block::default().borders(Borders::ALL);
    let area = Rect::new(0, 0, 40, 10);
    let inner = block.inner(area);

    let text = "Line 1\nLine 2\nLine 3\nThis is a long sentence that should easily wrap across multiple lines at 40 width.";
    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    let count = para.line_count(inner.width);
    assert!(count >= 4);
}

#[tokio::test]
async fn test_chat_autoscroll_lifecycle() {
    let mut app = App::new("test-workspace".to_string(), "gpt-4o-mini".to_string(), None);
    assert!(app.auto_scroll);
    assert_eq!(app.scroll_offset, 0);

    // Simulate chat viewport with max_scroll = 50
    app.max_scroll = 50;

    // Scroll up pauses autoscroll
    app.scroll_offset = 30;
    app.scroll_up(5);
    assert_eq!(app.scroll_offset, 25);
    assert!(!app.auto_scroll);

    // Scroll down moves closer to bottom
    app.scroll_down(10);
    assert_eq!(app.scroll_offset, 35);
    assert!(!app.auto_scroll);

    // Reaching bottom resumes autoscroll
    app.scroll_down(20);
    assert_eq!(app.scroll_offset, 50);
    assert!(app.auto_scroll);

    // Scroll to top
    app.scroll_to_top();
    assert_eq!(app.scroll_offset, 0);
    assert!(!app.auto_scroll);

    // Scroll to bottom
    app.scroll_to_bottom();
    assert_eq!(app.scroll_offset, 50);
    assert!(app.auto_scroll);

    // Submitting input snaps to bottom and sets auto_scroll = true
    app.scroll_up(10);
    assert!(!app.auto_scroll);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    app.input_buffer = "hello".to_string();
    app.submit_input(tx);
    assert!(app.auto_scroll);
    assert_eq!(app.scroll_offset, 50);
}



