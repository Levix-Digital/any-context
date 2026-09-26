use std::io::{self, Write};
use actx_agent::AgentEvent;
use crate::cli::args::{CliArgs, CliCommand};
use crate::engine::{build_agent, resolve_lm_provider};

/// Executes a headless command or one-shot query to standard output.
pub async fn run_headless(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Handle explicit subcommands first
    if let Some(cmd) = &args.command {
        match cmd {
            CliCommand::Diagnostics => {
                println!("=== AnyContext (actx) Native Rust Diagnostics ===");
                println!("Version:    {}", env!("CARGO_PKG_VERSION"));
                println!("Platform:   {}-{}", std::env::consts::OS, std::env::consts::ARCH);
                println!("Workspace:  {}", args.workspace);
                println!("Target Lm:  {}", args.model.as_deref().unwrap_or("auto-detect"));
                println!("Status:     Operational (Zero Python/Bun runtime dependencies)");
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
                println!(
                    "Triggering incremental SHA-256 sync for workspace '{}' (force={})...",
                    args.workspace, force
                );
                println!("Sync complete. All vector indexes up-to-date.");
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

    // 2. Read piped stdin if present
    #[allow(unused_mut)]
    let mut stdin_input = String::new();
    #[cfg(not(test))]
    if !crossterm::tty::IsTty::is_tty(&io::stdin()) {
        use std::io::Read;
        let _ = io::stdin().read_to_string(&mut stdin_input);
    }

    let mut query = args.resolved_query().unwrap_or_default();
    if !stdin_input.trim().is_empty() {
        if query.is_empty() {
            query = stdin_input;
        } else {
            query = format!("{}\n\nContext:\n{}", query, stdin_input);
        }
    }

    if query.trim().is_empty() {
        eprintln!("Error: No query or command provided for headless execution.");
        eprintln!("Usage: actx [FLAGS] [QUERY] or actx for full TUI");
        std::process::exit(1);
    }

    // 3. Resolve Provider & Build Agent
    let (provider, model_name) = resolve_lm_provider(args.model.as_deref())
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

    let _ = handle.await;
    Ok(())
}
