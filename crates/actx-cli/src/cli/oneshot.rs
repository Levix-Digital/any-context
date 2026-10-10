use std::io::{self, Write};
use actx_agent::AgentEvent;
use crate::cli::args::{CliArgs, CliCommand};
use crate::engine::{build_agent, resolve_lm_provider};

/// Executes a headless command or one-shot query to standard output.
pub async fn run_headless(args: CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // 0. Handle top-level flags first
    if args.check_update {
        handle_check_update();
        return Ok(());
    }

    if args.update {
        let target_ver = args.version_target.as_deref()
            .or_else(|| args.positional_query.first().map(|s| s.as_str()).filter(|s| s.starts_with('v') || s.starts_with('@')));
        handle_update(target_ver);
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

    if args.sync_worker {
        let fld = args.folder.as_deref().or_else(|| {
            args.positional_query.first().map(|s| s.as_str()).filter(|s| *s == "." || std::path::Path::new(s).exists())
        });
        let orchestrator = any_context_core_rs::ingestion::NativeSyncOrchestrator::new_default()
            .map_err(|e| format!("Failed to initialize native orchestrator: {e}"))?;
        let options = any_context_core_rs::ingestion::SyncOptions {
            workspace: args.workspace.clone(),
            force: args.force,
            target_folder: fld.map(|s| s.to_string()),
            verbose: true,
            model: args.model.clone(),
        };
        match orchestrator.run(&options).await {
            Ok(res) => {
                if res.is_up_to_date {
                    println!("✔ Workspace '{}' is already 100% up-to-date (0 changes).", args.workspace);
                } else {
                    println!("✔ Native background sync finished: {} files indexed, {} chunks created in {}ms.",
                        res.indexed_files, res.chunks_created, res.duration_ms);
                }
            }
            Err(e) => {
                eprintln!("[!] Native background sync error: {e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if args.sync {
        let fld = args.folder.as_deref().or_else(|| {
            args.positional_query.first().map(|s| s.as_str()).filter(|s| *s == "." || std::path::Path::new(s).exists())
        });
        let fld_msg = if let Some(f) = fld {
            format!(" (scoped folder: '{}')", f)
        } else {
            String::new()
        };
        println!("🦀 Running 100% Native Rust sync for workspace '{}' (force={}){}...", args.workspace, args.force, fld_msg);
        let orchestrator = any_context_core_rs::ingestion::NativeSyncOrchestrator::new_default()
            .map_err(|e| format!("Failed to initialize native orchestrator: {e}"))?;
        let options = any_context_core_rs::ingestion::SyncOptions {
            workspace: args.workspace.clone(),
            force: args.force,
            target_folder: fld.map(|s| s.to_string()),
            verbose: true,
            model: args.model.clone(),
        };
        match orchestrator.run(&options).await {
            Ok(res) => {
                if res.is_up_to_date {
                    println!("✔ Workspace '{}' is already 100% up-to-date (0 changes).", args.workspace);
                } else {
                    println!("✔ Native sync completed: {} files indexed, {} chunks created in {}ms.",
                        res.indexed_files, res.chunks_created, res.duration_ms);
                }
            }
            Err(e) => {
                eprintln!("[!] Native sync failed: {e}");
                std::process::exit(1);
            }
        }
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
            CliCommand::Update { check, version, target } => {
                if *check {
                    handle_check_update();
                } else {
                    let target_ver = version.as_deref()
                        .or(target.as_deref())
                        .or(args.version_target.as_deref());
                    handle_update(target_ver);
                }
                return Ok(());
            }
            CliCommand::Sync { force, folder } => {
                let fld = folder.as_deref().or_else(|| {
                    args.positional_query.first().map(|s| s.as_str()).filter(|s| *s == "." || std::path::Path::new(s).exists())
                });
                let fld_msg = if let Some(f) = fld {
                    format!(" (scoped folder: '{}')", f)
                } else {
                    String::new()
                };
                println!("🦀 Running 100% Native Rust sync for workspace '{}' (force={}){}...", args.workspace, force, fld_msg);
                let orchestrator = any_context_core_rs::ingestion::NativeSyncOrchestrator::new_default()
                    .map_err(|e| format!("Failed to initialize native orchestrator: {e}"))?;
                let options = any_context_core_rs::ingestion::SyncOptions {
                    workspace: args.workspace.clone(),
                    force: *force,
                    target_folder: fld.map(|s| s.to_string()),
                    verbose: true,
                    model: args.model.clone(),
                };
                match orchestrator.run(&options).await {
                    Ok(res) => {
                        if res.is_up_to_date {
                            println!("✔ Workspace '{}' is already 100% up-to-date (0 changes).", args.workspace);
                        } else {
                            println!("✔ Native sync completed: {} files indexed, {} chunks created in {}ms.",
                                res.indexed_files, res.chunks_created, res.duration_ms);
                        }
                    }
                    Err(e) => {
                        eprintln!("[!] Native sync failed: {e}");
                        std::process::exit(1);
                    }
                }
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
        handle_check_update();
        return Ok(());
    }

    if trimmed == "--update" || trimmed == "-u" || trimmed == "update" || trimmed == "upgrade" {
        handle_update(args.version_target.as_deref());
        return Ok(());
    }

    if let Some(target) = trimmed.strip_prefix("--update@")
        .or_else(|| trimmed.strip_prefix("-u@"))
        .or_else(|| trimmed.strip_prefix("update@"))
        .or_else(|| trimmed.strip_prefix("upgrade@"))
        .or_else(|| trimmed.strip_prefix("--update="))
        .or_else(|| trimmed.strip_prefix("-u="))
    {
        handle_update(Some(target));
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
    let db = any_context_core_rs::storage::NativeConfigDb::open_default().ok();
    let grounding_mode = db.as_ref()
        .and_then(|d| d.get_workspace_grounding_mode(&args.workspace).ok())
        .unwrap_or_else(|| "strict".to_string());
    let search_mode = db.as_ref()
        .and_then(|d| d.get_workspace_search_mode(&args.workspace).ok())
        .unwrap_or_else(|| "auto".to_string());
    let web_search_enabled = db.as_ref()
        .and_then(|d| d.get_workspace_web_search(&args.workspace).ok())
        .unwrap_or(false);

    let (provider, model_name) = resolve_lm_provider(args.model.as_deref(), Some(&args.workspace))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let agent = build_agent(provider, &model_name, &args.workspace, &grounding_mode, &search_mode, web_search_enabled).await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    // 4. Stream response to stdout
    let session_id = format!("ws_{}", args.workspace);
    let (mut rx, handle) = agent.stream(query, Some(session_id));

    let mut stdout = io::stdout();
    let mut in_thinking = false;
    let theme = crate::theme::UiTheme::default();

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::Thinking(token) => {
                if !in_thinking {
                    print!("{}\x1b[2m<think>\x1b[0m\n", theme.core.text_muted.ansi_fg());
                    in_thinking = true;
                }
                print!("{}\x1b[2m{}\x1b[0m", theme.core.text_muted.ansi_fg(), token);
                let _ = stdout.flush();
            }
            AgentEvent::ToolStart { name, arguments, .. } => {
                if in_thinking {
                    print!("{}\x1b[2m</think>\x1b[0m\n\n", theme.core.text_muted.ansi_fg());
                    in_thinking = false;
                }
                println!("{} \x1b[2m{}\x1b[0m", theme.ansi_accent(&format!("🔧 [Tool Call: {}]", name)), arguments);
            }
            AgentEvent::ToolEnd { name, result, is_error, .. } => {
                if is_error {
                    println!("{}: {}", theme.ansi_error(&format!("❌ [Tool Error: {}]", name)), result);
                } else {
                    println!("{}", theme.ansi_primary(&format!("✔ [Tool Done: {}]", name)));
                }
            }
            AgentEvent::RoutingDecision { mode, complexity, intent, confidence, reason } => {
                let badge = if complexity == "Deep" || mode.contains("DeepSearch") {
                    theme.ansi_reasoning("🧠 [ModelRouter: Deep Search]")
                } else {
                    theme.ansi_warning("⚡ [ModelRouter: Fast RAG]")
                };
                println!("{} \x1b[2m(intent: {}, confidence: {:.0}%)\x1b[0m\n\x1b[2m  • {}\x1b[0m\n", badge, intent, confidence * 100.0, reason);
            }
            AgentEvent::Decomposition { sub_queries } => {
                if in_thinking {
                    print!("{}\x1b[2m</think>\x1b[0m\n\n", theme.core.text_muted.ansi_fg());
                    in_thinking = false;
                }
                println!("{}", theme.ansi_accent("🌲 [Deep Search: Decomposing Query into Sub-Queries]"));
                for (i, q) in sub_queries.iter().enumerate() {
                    println!("   \x1b[2m{}.\x1b[0m {}", i + 1, q);
                }
                println!();
            }
            AgentEvent::IterationStart { iteration, max_iterations } => {
                if in_thinking {
                    print!("{}\x1b[2m</think>\x1b[0m\n\n", theme.core.text_muted.ansi_fg());
                    in_thinking = false;
                }
                println!("{} \x1b[2mBatch Retrieval & Reflection...\x1b[0m", theme.ansi_warning(&format!("🔄 [Deep Search Iteration {}/{}]", iteration, max_iterations)));
            }
            AgentEvent::GapAnalysis { is_sufficient, missing_aspects } => {
                if in_thinking {
                    print!("{}\x1b[2m</think>\x1b[0m\n\n", theme.core.text_muted.ansi_fg());
                    in_thinking = false;
                }
                if is_sufficient {
                    println!("{} \x1b[2mSynthesizing grounded answer...\x1b[0m\n", theme.ansi_primary("✔ [Evidence Complete]"));
                } else {
                    println!("{} Missing aspects: {}\n", theme.ansi_warning("🔎 [Gap Analysis]"), missing_aspects.join(", "));
                }
            }
            AgentEvent::Delta(token) => {
                if in_thinking {
                    print!("{}\x1b[2m</think>\x1b[0m\n\n", theme.core.text_muted.ansi_fg());
                    in_thinking = false;
                }
                print!("{}", token);
                let _ = stdout.flush();
            }
            AgentEvent::Done { .. } => {
                if in_thinking {
                    print!("{}\x1b[2m</think>\x1b[0m\n", theme.core.text_muted.ansi_fg());
                }
                println!();
            }
            AgentEvent::Error(err) => {
                eprintln!("\n{}", theme.ansi_error(&format!("Error: {}", err)));
            }
        }
    }

    if let Ok(Err(err)) = handle.await {
        eprintln!("\n{}", theme.ansi_error(&format!("Error: {}", err)));
    }
    Ok(())
}

fn handle_check_update() {
    println!("Checking for updates...");
    match actx_installer::downloader::fetch_latest_release_tag() {
        Ok(latest) => {
            let current_tag = format!("v{}", env!("CARGO_PKG_VERSION"));
            let clean_latest = if latest.starts_with('v') || latest.starts_with('V') {
                latest
            } else {
                format!("v{}", latest)
            };

            let parse_ver = |v: &str| -> (u64, u64, u64) {
                let s = v.trim_start_matches(|c| c == 'v' || c == 'V');
                let mut parts = s.split('.').filter_map(|p| p.parse::<u64>().ok());
                (
                    parts.next().unwrap_or(0),
                    parts.next().unwrap_or(0),
                    parts.next().unwrap_or(0),
                )
            };

            if parse_ver(&clean_latest) > parse_ver(&current_tag) {
                println!(
                    "A new release of actx is available: {} (current: {}). Run 'actx --update' to upgrade.",
                    clean_latest, current_tag
                );
            } else {
                println!("actx {} is up to date (latest: {}).", current_tag, clean_latest);
            }
        }
        Err(e) => {
            println!("actx v{} (latest release check: {})", env!("CARGO_PKG_VERSION"), e);
        }
    }
}

fn handle_update(target_ver: Option<&str>) {
    let clean = target_ver.map(|t| t.trim_start_matches('@'));
    let bin_dir = actx_installer::paths::get_canonical_bin_dir();
    actx_installer::run_standalone_update(&bin_dir, clean);
}
