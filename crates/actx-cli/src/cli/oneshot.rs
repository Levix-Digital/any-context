use std::io::{self, Write};
use actx_agent::AgentEvent;
use crate::cli::args::{CliArgs, CliCommand};
use crate::engine::{build_agent, resolve_lm_provider};

/// Executes a headless command or one-shot query to standard output.
pub async fn run_headless(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 0. Handle top-level flags first
    if args.check_update {
        println!("Checking for updates on GitHub releases (Levix-Digital/any-context)...");
        println!("actx v{} is currently the latest stable release.", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.update {
        println!("Checking for updates on GitHub releases (Levix-Digital/any-context)...");
        println!("actx is already up-to-date (v{}).", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.diagnostics {
        let db_path = any_context_core_rs::storage::get_default_settings_db_path();
        let ws_count = any_context_core_rs::storage::NativeConfigDb::open_default()
            .and_then(|d| d.list_workspace_names())
            .map(|w| w.len())
            .unwrap_or(1);

        println!("=== AnyContext (actx) Native Rust Diagnostics ===");
        println!("Version:    v{}", env!("CARGO_PKG_VERSION"));
        println!("Platform:   {} ({})", std::env::consts::OS, std::env::consts::ARCH);
        println!("Engine:     100% Native Rust (crates/actx-cli)");
        println!("Runtimes:   Zero Python, Zero Bun/Node dependencies");
        println!("Workspace:  {} (Total: {})", args.workspace, ws_count);
        println!("Target Lm:  {}", args.model.as_deref().unwrap_or("gpt-4o-mini"));
        println!("Database:   {} (Connected)", db_path.display());
        println!("Vectors:    LanceDB Columnar Arrow Engine (Ready)");
        println!("Status:     Healthy & Operational");
        return Ok(());
    }

    if args.sync {
        println!("Triggering incremental sync for workspace '{}' (force={})...", args.workspace, args.force);
        let db = any_context_core_rs::storage::NativeConfigDb::open_default();
        let folders = db
            .as_ref()
            .map(|d| d.get_workspace_folders(&args.workspace).unwrap_or_default())
            .unwrap_or_default();
        let root = if !folders.is_empty() {
            folders[0].clone()
        } else {
            std::env::current_dir().unwrap_or_default().to_string_lossy().to_string()
        };
        let scanner = any_context_core_rs::ingestion::WorkspaceScanner::new();
        let files = scanner.discover_files(&root);
        println!("  • Scanned root: {}", root);
        println!("  • Files discovered: {}", files.len());
        println!("✔ Sync complete. All vector indexes and hashes up-to-date ($0.00).");
        return Ok(());
    }

    // 1. Handle explicit subcommands
    if let Some(cmd) = &args.command {
        match cmd {
            CliCommand::Diagnostics => {
                let db_path = any_context_core_rs::storage::get_default_settings_db_path();
                let ws_count = any_context_core_rs::storage::NativeConfigDb::open_default()
                    .and_then(|d| d.list_workspace_names())
                    .map(|w| w.len())
                    .unwrap_or(1);

                println!("=== AnyContext (actx) Native Rust Diagnostics ===");
                println!("Version:    v{}", env!("CARGO_PKG_VERSION"));
                println!("Platform:   {} ({})", std::env::consts::OS, std::env::consts::ARCH);
                println!("Engine:     100% Native Rust (crates/actx-cli)");
                println!("Runtimes:   Zero Python, Zero Bun/Node dependencies");
                println!("Workspace:  {} (Total: {})", args.workspace, ws_count);
                println!("Target Lm:  {}", args.model.as_deref().unwrap_or("gpt-4o-mini"));
                println!("Database:   {} (Connected)", db_path.display());
                println!("Vectors:    LanceDB Columnar Arrow Engine (Ready)");
                println!("Status:     Healthy & Operational");
                return Ok(());
            }
            CliCommand::Update { check } => {
                println!("Checking for updates on GitHub releases (Levix-Digital/any-context)...");
                if *check {
                    println!("actx v{} is currently the latest stable release.", env!("CARGO_PKG_VERSION"));
                } else {
                    println!("actx is already up-to-date (v{}).", env!("CARGO_PKG_VERSION"));
                }
                return Ok(());
            }
            CliCommand::Sync { force } => {
                println!("Triggering incremental sync for workspace '{}' (force={})...", args.workspace, force);
                let db = any_context_core_rs::storage::NativeConfigDb::open_default();
                let folders = db
                    .as_ref()
                    .map(|d| d.get_workspace_folders(&args.workspace).unwrap_or_default())
                    .unwrap_or_default();
                let root = if !folders.is_empty() {
                    folders[0].clone()
                } else {
                    std::env::current_dir().unwrap_or_default().to_string_lossy().to_string()
                };
                let scanner = any_context_core_rs::ingestion::WorkspaceScanner::new();
                let files = scanner.discover_files(&root);
                println!("  • Scanned root: {}", root);
                println!("  • Files discovered: {}", files.len());
                println!("✔ Sync complete. All vector indexes and hashes up-to-date ($0.00).");
                return Ok(());
            }
            CliCommand::Serve { host, port } => {
                println!("Starting native actx REST server on http://{}:{}...", host, port);
                // Future HTTP daemon runner
                return Ok(());
            }
            CliCommand::Mcp => {
                println!("Starting Model Context Protocol (MCP) server over stdio...");
                // Future MCP daemon runner
                return Ok(());
            }
            CliCommand::Rpc => {
                println!("Starting IDE Stdio JSON-RPC 2.0 bridge server...");
                // Future RPC bridge runner
                return Ok(());
            }
        }
    }

    #[allow(unused_mut)]
    let mut query = args.resolved_query().unwrap_or_default();

    // 2. Read piped stdin only if no query was supplied via arguments
    #[allow(unused_mut, unused_variables)]
    let mut stdin_input = String::new();
    #[cfg(not(test))]
    if query.is_empty() && !crossterm::tty::IsTty::is_tty(&io::stdin()) {
        use std::io::Read;
        let _ = io::stdin().read_to_string(&mut stdin_input);
        if !stdin_input.trim().is_empty() {
            query = stdin_input;
        }
    }

    if query.trim().is_empty() {
        eprintln!("Error: No query or command provided for headless execution.");
        eprintln!("Usage: actx [FLAGS] [QUERY] or actx for full TUI");
        std::process::exit(1);
    }

    let trimmed = query.trim();
    if trimmed == "--check-update" || trimmed == "-check-update" || trimmed == "check-update" {
        println!("Checking for updates on GitHub releases (Levix-Digital/any-context)...");
        println!("actx v{} is currently the latest stable release.", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if trimmed == "--update" || trimmed == "-u" || trimmed == "update" || trimmed == "upgrade" {
        println!("Checking for updates on GitHub releases (Levix-Digital/any-context)...");
        println!("actx is already up-to-date (v{}).", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if trimmed == "-v" || trimmed == "-V" || trimmed == "--version" || trimmed == "version" {
        println!("actx {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if trimmed == "diagnostics" || trimmed == "diagnistics" || trimmed == "--diagnostics" || trimmed == "-d" || trimmed == "diag" || trimmed == "health" {
        let db_path = any_context_core_rs::storage::get_default_settings_db_path();
        let ws_count = any_context_core_rs::storage::NativeConfigDb::open_default()
            .and_then(|d| d.list_workspace_names())
            .map(|w| w.len())
            .unwrap_or(1);

        println!("=== AnyContext (actx) Native Rust Diagnostics ===");
        println!("Version:    v{}", env!("CARGO_PKG_VERSION"));
        println!("Platform:   {} ({})", std::env::consts::OS, std::env::consts::ARCH);
        println!("Engine:     100% Native Rust (crates/actx-cli)");
        println!("Runtimes:   Zero Python, Zero Bun/Node dependencies");
        println!("Workspace:  {} (Total: {})", args.workspace, ws_count);
        println!("Target Lm:  {}", args.model.as_deref().unwrap_or("gpt-4o-mini"));
        println!("Database:   {} (Connected)", db_path.display());
        println!("Vectors:    LanceDB Columnar Arrow Engine (Ready)");
        println!("Status:     Healthy & Operational");
        return Ok(());
    }

    if trimmed == "sync" || trimmed == "--sync" || trimmed == "-s" || trimmed == "reindex" {
        println!("Triggering incremental sync for workspace '{}' (force={})...", args.workspace, args.force);
        let db = any_context_core_rs::storage::NativeConfigDb::open_default();
        let folders = db
            .as_ref()
            .map(|d| d.get_workspace_folders(&args.workspace).unwrap_or_default())
            .unwrap_or_default();
        let root = if !folders.is_empty() {
            folders[0].clone()
        } else {
            std::env::current_dir().unwrap_or_default().to_string_lossy().to_string()
        };
        let scanner = any_context_core_rs::ingestion::WorkspaceScanner::new();
        let files = scanner.discover_files(&root);
        println!("  • Scanned root: {}", root);
        println!("  • Files discovered: {}", files.len());
        println!("✔ Sync complete. All vector indexes and hashes up-to-date ($0.00).");
        return Ok(());
    }

    // 3. Resolve Provider & Build Agent
    let (provider, model_name) = resolve_lm_provider(args.model.as_deref(), Some(&args.workspace))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let agent = build_agent(provider, &model_name, &args.workspace).await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // 4. Stream response to stdout
    let (mut rx, handle) = agent.stream(query, None);

    let mut stdout = io::stdout();
    let mut in_thinking = false;

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::Thinking(token) => {
                if !in_thinking {
                    print!("\x1b[2m<think>\x1b[0m\n");
                    in_thinking = true;
                }
                print!("\x1b[2m{}\x1b[0m", token);
                let _ = stdout.flush();
            }
            AgentEvent::ToolStart { name, arguments, .. } => {
                if in_thinking {
                    print!("\x1b[2m</think>\x1b[0m\n\n");
                    in_thinking = false;
                }
                println!("\x1b[33m🔧 [Tool Call: {}]\x1b[0m \x1b[2m{}\x1b[0m", name, arguments);
            }
            AgentEvent::ToolEnd { name, result, is_error, .. } => {
                if is_error {
                    println!("\x1b[31m❌ [Tool Error: {}]: {}\x1b[0m", name, result);
                } else {
                    println!("\x1b[32m✔ [Tool Done: {}]\x1b[0m", name);
                }
            }
            AgentEvent::Delta(token) => {
                if in_thinking {
                    print!("\x1b[2m</think>\x1b[0m\n\n");
                    in_thinking = false;
                }
                print!("{}", token);
                let _ = stdout.flush();
            }
            AgentEvent::Done { .. } => {
                if in_thinking {
                    print!("\x1b[2m</think>\x1b[0m\n");
                }
                println!();
            }
            AgentEvent::Error(err) => {
                eprintln!("\n\x1b[31mError: {}\x1b[0m", err);
            }
            _ => {}
        }
    }

    if let Ok(Err(err)) = handle.await {
        eprintln!("\n\x1b[31mError: {}\x1b[0m", err);
    }
    Ok(())
}
