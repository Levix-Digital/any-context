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
    assert!(matches.iter().any(|c| c.name == "switch"));
}

fn setup_test_sandbox() {
    std::env::set_var("ACTX_TEST_MODE", "1");
}

#[tokio::test]
async fn test_app_state_and_slash_dispatch() {
    setup_test_sandbox();
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

    // Test /mode hybrid
    app.input_buffer = "/mode hybrid".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let mode_msg = app.chat_history.last().expect("mode item");
    assert!(mode_msg.content.contains("Grounding Strategy Mode"));
    assert_eq!(app.grounding_mode, "hybrid");

    // Test /search deep
    app.input_buffer = "/search deep".to_string();
    app.cursor_idx = app.input_buffer.len();
    app.submit_input(tx.clone());
    let search_msg = app.chat_history.last().expect("search item");
    assert!(search_msg.content.contains("Search Depth Policy"));
    assert_eq!(app.search_mode, "deep");

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
    setup_test_sandbox();
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
    setup_test_sandbox();
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

#[tokio::test]
async fn test_prompt_history_navigation_and_workspace_isolation() {
    setup_test_sandbox();
    let mut app = App::new("WorkspaceA".to_string(), "gpt-4o-mini".to_string(), None);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    // 1. Submit 3 prompts in WorkspaceA
    app.input_buffer = "prompt 1".to_string();
    app.submit_input(tx.clone());

    app.input_buffer = "prompt 2".to_string();
    app.submit_input(tx.clone());

    app.input_buffer = "prompt 3".to_string();
    app.submit_input(tx.clone());

    // 2. User starts typing a draft
    app.input_buffer = "my unsubmitted draft".to_string();
    app.cursor_idx = app.input_buffer.len();

    // 3. Press Up: should recall "prompt 3"
    app.palette_up();
    assert_eq!(app.input_buffer, "prompt 3");
    assert_eq!(app.history_index, Some(2));

    // Press Up again: should recall "prompt 2"
    app.palette_up();
    assert_eq!(app.input_buffer, "prompt 2");
    assert_eq!(app.history_index, Some(1));

    // Press Up again: should recall "prompt 1"
    app.palette_up();
    assert_eq!(app.input_buffer, "prompt 1");
    assert_eq!(app.history_index, Some(0));

    // Press Up at start: remains "prompt 1"
    app.palette_up();
    assert_eq!(app.input_buffer, "prompt 1");

    // Press Down: should advance to "prompt 2"
    app.palette_down();
    assert_eq!(app.input_buffer, "prompt 2");
    assert_eq!(app.history_index, Some(1));

    // Press Down: should advance to "prompt 3"
    app.palette_down();
    assert_eq!(app.input_buffer, "prompt 3");
    assert_eq!(app.history_index, Some(2));

    // Press Down at end: restores "my unsubmitted draft"
    app.palette_down();
    assert_eq!(app.input_buffer, "my unsubmitted draft");
    assert_eq!(app.history_index, None);

    // 4. Switch workspace to WorkspaceB
    app.input_buffer = "/switch WorkspaceB".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.active_workspace, "WorkspaceB");

    // Press Up in WorkspaceB: history is empty, input buffer stays empty
    app.input_buffer.clear();
    app.palette_up();
    assert_eq!(app.input_buffer, "");
}

#[tokio::test]
async fn test_top_header_quint_status_and_reactive_updates() {
    setup_test_sandbox();
    let mut app = App::new("Default".to_string(), "gpt-4o-mini".to_string(), None);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    // Verify initial values exist
    assert!(!app.grounding_mode.is_empty());
    assert!(!app.search_mode.is_empty());

    // Switch grounding mode to hybrid
    app.input_buffer = "/mode hybrid".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.grounding_mode, "hybrid");

    // Switch grounding mode to proactive
    app.input_buffer = "/mode proactive".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.grounding_mode, "proactive");

    // Switch grounding mode to strict
    app.input_buffer = "/mode strict".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.grounding_mode, "strict");

    // Switch search depth mode to fast
    app.input_buffer = "/search fast".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.search_mode, "fast");

    // Switch search depth mode to deep
    app.input_buffer = "/search deep".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.search_mode, "deep");

    // Switch search depth mode to auto
    app.input_buffer = "/search auto".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.search_mode, "auto");

    // Enable web search
    app.input_buffer = "/web-search on".to_string();
    app.submit_input(tx.clone());
    assert!(app.web_search_enabled);

    // Disable web search
    app.input_buffer = "/web-search off".to_string();
    app.submit_input(tx.clone());
    assert!(!app.web_search_enabled);
}

#[tokio::test]
async fn test_sqlite_session_store_integration() {
    use actx_agent::SessionStore;
    use actx_lm::types::ChatMessage;

    let temp_db = std::env::temp_dir().join(format!("actx_test_sess_{}.db", std::process::id()));
    let store = actx_agent::SqliteSessionStore::open(&temp_db, 50).expect("open session store");

    let session_id = "ws_test_integration";

    // 1. Initially empty
    let msgs = store.get_messages(session_id).await.expect("get messages");
    assert!(msgs.is_empty());

    // 2. Append turn
    let user_msg = ChatMessage::user("What is the speed of light?");
    let asst_msg = ChatMessage::assistant("Approximately 299,792,458 m/s.");
    store.append_messages(session_id, &[user_msg, asst_msg]).await.expect("append");

    // 3. Verify retrieval
    let retrieved = store.get_messages(session_id).await.expect("get messages");
    assert_eq!(retrieved.len(), 2);
    assert_eq!(retrieved[0].content, "What is the speed of light?");
    assert_eq!(retrieved[1].content, "Approximately 299,792,458 m/s.");

    // 4. Clear session
    store.clear_session(session_id).await.expect("clear");
    let cleared = store.get_messages(session_id).await.expect("get messages");
    assert!(cleared.is_empty());

    let _ = std::fs::remove_file(temp_db);
}

#[tokio::test]
async fn test_workspace_chat_buffers_isolation_and_clear_lifecycle() {
    setup_test_sandbox();
    let mut app = App::new("WorkspaceAlpha".to_string(), "gpt-4o-mini".to_string(), None);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    // 1. Submit prompts in WorkspaceAlpha
    app.input_buffer = "Question in Alpha 1".to_string();
    app.submit_input(tx.clone());

    app.input_buffer = "Question in Alpha 2".to_string();
    app.submit_input(tx.clone());

    // Verify Alpha chat history contains both questions
    assert!(app.chat_history.iter().any(|m| m.content == "Question in Alpha 1"));
    assert!(app.chat_history.iter().any(|m| m.content == "Question in Alpha 2"));

    // 2. Switch to WorkspaceBeta
    app.input_buffer = "/switch WorkspaceBeta".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.active_workspace, "WorkspaceBeta");

    // CRITICAL: WorkspaceBeta MUST NOT show Alpha's messages
    assert!(!app.chat_history.iter().any(|m| m.content == "Question in Alpha 1"), "Alpha messages leaked into Beta!");
    assert!(!app.chat_history.iter().any(|m| m.content == "Question in Alpha 2"), "Alpha messages leaked into Beta!");
    assert!(app.chat_history.iter().any(|m| m.content.contains("WorkspaceBeta")));

    // 3. Submit prompt in WorkspaceBeta
    app.input_buffer = "Question in Beta 1".to_string();
    app.submit_input(tx.clone());
    assert!(app.chat_history.iter().any(|m| m.content == "Question in Beta 1"));

    // 4. Switch back to WorkspaceAlpha
    app.input_buffer = "/switch WorkspaceAlpha".to_string();
    app.submit_input(tx.clone());
    assert_eq!(app.active_workspace, "WorkspaceAlpha");

    // CRITICAL: WorkspaceAlpha MUST restore Alpha's messages and MUST NOT contain Beta's messages
    assert!(app.chat_history.iter().any(|m| m.content == "Question in Alpha 1"), "Alpha message 1 not restored!");
    assert!(app.chat_history.iter().any(|m| m.content == "Question in Alpha 2"), "Alpha message 2 not restored!");
    assert!(!app.chat_history.iter().any(|m| m.content == "Question in Beta 1"), "Beta messages leaked into Alpha!");

    // 5. Execute /clear in WorkspaceAlpha
    app.input_buffer = "/clear".to_string();
    app.submit_input(tx.clone());
    assert!(app.chat_history.is_empty(), "Chat history was not cleared by /clear!");

    // 6. Switch back to WorkspaceBeta: Beta MUST still have its message!
    app.input_buffer = "/switch WorkspaceBeta".to_string();
    app.submit_input(tx.clone());
    assert!(app.chat_history.iter().any(|m| m.content == "Question in Beta 1"), "Beta messages affected by Alpha's /clear!");

    // 7. Execute /reset-memory in WorkspaceBeta: resets both SQLite memory and view buffer
    app.input_buffer = "/reset-memory".to_string();
    app.submit_input(tx.clone());
    assert!(!app.chat_history.iter().any(|m| m.content == "Question in Beta 1"), "Beta view buffer not cleared by /reset-memory!");
    assert!(app.chat_history.iter().any(|m| m.content.contains("Long-term session memory reset")), "Reset confirmation missing!");
}

#[test]
fn test_clean_screen_startup_and_long_term_memory_preservation() {
    setup_test_sandbox();
    let db_path = any_context_core_rs::storage::get_default_settings_db_path();
    let store = actx_agent::SqliteSessionStore::open(&db_path, 50).expect("open session store");
    let session_id = "ws_CleanScreenWS";

    // 1. Populate SQLite with past session messages
    use actx_lm::types::ChatMessage;
    let u_msg = ChatMessage::user("What was the result of yesterday's query?");
    let a_msg = ChatMessage::assistant("The result was 42.");
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        use actx_agent::SessionStore;
        store.append_messages(session_id, &[u_msg, a_msg]).await.expect("append messages");
    });

    // Verify messages exist in SQLite long-term storage
    let stored = store.get_messages_sync(session_id).expect("get stored");
    assert_eq!(stored.len(), 2, "SQLite must have 2 messages in long-term memory");

    // 2. Start a brand new App session
    let mut app = App::new("CleanScreenWS".to_string(), "gpt-4o-mini".to_string(), None);
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

    // CRITICAL: Chat history MUST start clean with ONLY the welcome message banner
    assert_eq!(app.chat_history.len(), 1, "App chat_history must contain only the initial welcome banner on startup");
    assert!(!app.chat_history.iter().any(|m| m.content.contains("What was the result of yesterday's query?")), "Old session leaked onto screen buffer!");
    assert!(!app.chat_history.iter().any(|m| m.content.contains("The result was 42.")), "Old session leaked onto screen buffer!");

    // 3. Long-term memory is still safely preserved in SQLite
    let still_stored = store.get_messages_sync(session_id).expect("get stored");
    assert_eq!(still_stored.len(), 2, "Long-term memory must remain intact in SQLite");

    // 4. Test /history command inspecting long-term memory
    app.input_buffer = "/history".to_string();
    app.submit_input(tx.clone());
    let hist_msg = app.chat_history.last().expect("history msg");
    assert!(hist_msg.content.contains("Long-term Session Memory for 'CleanScreenWS'"));
    assert!(hist_msg.content.contains("2 messages in SQLite"));
    assert!(hist_msg.content.contains("Screen buffer is clean for this session"));

    // 5. Test /history --clear wiping long-term memory
    app.input_buffer = "/history --clear".to_string();
    app.submit_input(tx);
    let after_clear = store.get_messages_sync(session_id).expect("get stored");
    assert!(after_clear.is_empty(), "SQLite session memory must be wiped by /history --clear");
}

#[test]
fn test_sync_progress_bar_telemetry_and_agent_status_tool() {
    setup_test_sandbox();
    let ws = format!("SyncTelemetryWS_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    let db = any_context_core_rs::storage::NativeConfigDb::open_default().expect("open db");

    // 1. Initially no sync status
    let initial_status = db.get_sync_status(&ws).expect("query status");
    assert!(initial_status.is_none());

    // 2. Publish active sync status
    db.update_sync_status(
        &ws,
        true,
        Some(12345),
        15,
        30,
        "files",
        Some("contract.pdf"),
        None,
    ).expect("update sync status");

    // Verify DB contains progress bar
    let status = db.get_sync_status(&ws).expect("query status").expect("status row");
    assert!(status.is_syncing);
    assert_eq!(status.current_item, 15);
    assert_eq!(status.total_items, 30);
    assert!(status.progress_bar.contains("50%"));
    assert!(status.progress_bar.contains("15/30 files"));

    // 3. App polls sync status and updates its state
    let mut app = App::new(ws.clone(), "gpt-4o-mini".to_string(), None);
    app.poll_sync_status();
    let app_status = app.sync_status.as_ref().expect("app sync status");
    assert!(app_status.is_syncing);
    assert!(app_status.progress_bar.contains("50%"));

    // 4. Update sync status to completed
    db.update_sync_status(
        &ws,
        false,
        Some(12345),
        30,
        30,
        "completed",
        None,
        None,
    ).expect("update sync status to completed");

    app.last_sync_poll = std::time::Instant::now() - std::time::Duration::from_secs(1);
    app.poll_sync_status();
    let completed_status = app.sync_status.as_ref().expect("completed status");
    assert!(!completed_status.is_syncing);
    assert!(completed_status.progress_bar.contains("100%"));
}

#[test]
fn test_menu_incremental_sync_selection_and_tick_animation() {
    let mut app = App::new("Default".to_string(), "gpt-4o-mini".to_string(), None);
    assert_eq!(app.tick_count, 0);
    app.tick();
    assert_eq!(app.tick_count, 1);
    app.tick();
    assert_eq!(app.tick_count, 2);

    // Open sync submenu
    app.open_sync_menu();
    assert!(app.menu_state.is_open);
    assert_eq!(app.menu_state.selected_item().unwrap().id, "sync_action:incremental");

    // Select incremental sync
    app.menu_select();
    assert!(!app.menu_state.is_open);

    // Verify system notification in chat history
    let last_msg = app.chat_history.last().expect("chat message after sync selection");
    assert_eq!(last_msg.role, actx_cli::tui::app::MessageRole::System);
    assert!(last_msg.content.contains("Synchronizing workspace") || last_msg.content.contains("worker spawned") || last_msg.content.contains("sync"));
}





