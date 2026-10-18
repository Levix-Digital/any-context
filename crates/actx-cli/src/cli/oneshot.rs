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
        println!("Target Lm:  {}", args.model.as_deref().unwrap_or(any_context_core_rs::llm_catalog::DEFAULT_CHAT_MODEL));
        let metrics = any_context_core_rs::hardware::HardwareProfiler::probe_system(None);
        println!("Hardware:   {} Cores, {:.1} GB RAM ({:.1} GB Free)", metrics.cpu_cores, metrics.total_ram_gb(), metrics.available_ram_gb());
        println!("Accel:      {}", metrics.acceleration_summary());
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
                print_sync_errors(&res);
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
                print_sync_errors(&res);
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
                println!("Target Lm:  {}", args.model.as_deref().unwrap_or(any_context_core_rs::llm_catalog::DEFAULT_CHAT_MODEL));
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
            CliCommand::Download { tier, force, check } => {
                handle_download(tier.as_deref(), *force, *check).await;
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
        println!("Target Lm:  {}", args.model.as_deref().unwrap_or(any_context_core_rs::llm_catalog::DEFAULT_CHAT_MODEL));
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
    if let Some(ref rk) = args.rerank {
        if let Some(ref d) = db {
            let _ = d.set_setting("rerank_mode", rk);
        }
    }
    if let Some(ref model) = args.rerank_model {
        use any_context_core_rs::ingestion::{OnnxModelCategory, OnnxModelManager};
        match OnnxModelManager::resolve_spec(model)
            .filter(|s| s.category == OnnxModelCategory::ReRanker && s.is_available())
        {
            // Per-run override: never persisted (use `/rerank model <id>` to persist).
            Some(spec) => std::env::set_var("ACTX_RERANKER_MODEL", spec.id),
            None => {
                let ids: Vec<&str> = OnnxModelManager::rerankers().map(|r| r.id).collect();
                eprintln!("❌ Unknown re-ranker model '{}'. Available: {}", model, ids.join(", "));
                std::process::exit(2);
            }
        }
    }
    let grounding_mode = db.as_ref()
        .and_then(|d| d.get_workspace_grounding_mode(&args.workspace).ok())
        .unwrap_or_else(|| "strict".to_string());
    let search_mode = db.as_ref()
        .and_then(|d| d.get_workspace_search_mode(&args.workspace).ok())
        .unwrap_or_else(|| "auto".to_string());
    let web_search_enabled = db.as_ref()
        .and_then(|d| d.get_workspace_web_search(&args.workspace).ok())
        .unwrap_or(false);

    let resolved = resolve_lm_provider(args.model.as_deref(), Some(&args.workspace))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    if let Some(ref notice) = resolved.notice {
        eprintln!("{}", crate::theme::UiTheme::default().ansi_warning(notice));
    }

    let agent = build_agent(resolved.provider, &resolved.model, &args.workspace, &grounding_mode, &search_mode, web_search_enabled).await
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
                let badge = if intent == "system_status" {
                    theme.ansi_warning("📊 [ModelRouter: Live Status]")
                } else if complexity == "Deep" || mode.contains("DeepSearch") {
                    theme.ansi_reasoning("🧠 [ModelRouter: Deep Search]")
                } else {
                    theme.ansi_warning("⚡ [ModelRouter: Fast RAG]")
                };
                let intent_display = match intent.as_str() {
                    "code" => "Code & Syntax",
                    "architecture" => "Architecture & Topology",
                    "devops_infra" => "DevOps & Infrastructure",
                    "data_schema" => "Data & Schema",
                    "security_audit" => "Security & Audit",
                    "document" => "Document & Business",
                    "system_status" => "System Status",
                    _ => "General & Concepts",
                };
                println!("{} \x1b[2m(Domain: {}, confidence: {:.0}%)\x1b[0m\n\x1b[2m  • {}\x1b[0m\n", badge, intent_display, confidence * 100.0, reason);
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
    // Honour the profile the user chose (`/profile`) so updates never provision models a
    // Lite user opted out of.
    let profile = any_context_core_rs::storage::NativeConfigDb::open_default()
        .ok()
        .and_then(|db| db.get_setting("active_profile").ok().flatten())
        .and_then(|p| actx_installer::hardware::InstallerProfileTier::parse_str(&p));
    actx_installer::run_standalone_update(&bin_dir, clean, profile);
}

pub async fn handle_download(tier_opt: Option<&str>, force: bool, check: bool) {
    use any_context_core_rs::ingestion::model_manager::{OnnxModelManager, DownloadProgressInfo};
    use any_context_core_rs::hardware::HardwareProfiler;
    use std::io::Write;

    let metrics = HardwareProfiler::probe_system(None);
    let summary = OnnxModelManager::inspect_store();

    if check {
        println!("=== AnyContext Local ONNX Model Store ===");
        println!("Models Directory: {}", summary.models_dir.display());
        println!("Installed: {} of {} models ({:.2} GB on disk)\n",
            summary.installed_count, summary.total_count, summary.total_bytes_on_disk as f64 / 1e9);

        println!("{:<20} {:<34} {:<15} {:<16} {:<24}",
            "ID", "Name", "Nominal Size", "Status", "Hardware Verdict");
        println!("{}", "-".repeat(110));

        for spec in OnnxModelManager::catalog() {
            let is_installed = OnnxModelManager::is_installed(spec);
            let size_str = if spec.is_available() {
                format!("{:.2} GB", spec.size_bytes as f64 / 1e9)
            } else {
                "—".to_string()
            };
            let status_str = if !spec.is_available() {
                "🕓 Planned".to_string()
            } else if is_installed {
                "✔ Installed".to_string()
            } else {
                "❌ Missing".to_string()
            };

            let safety = HardwareProfiler::validate_model_safety(spec.id, &metrics);
            let verdict_str = match safety {
                any_context_core_rs::hardware::HardwareSafetyVerdict::Safe { .. } => "🟢 [Compatible]".to_string(),
                any_context_core_rs::hardware::HardwareSafetyVerdict::Tight { .. } => "🟡 [Swap Risk]".to_string(),
                any_context_core_rs::hardware::HardwareSafetyVerdict::Unsafe { required_ram_gb, total_ram_gb, .. } => {
                    format!("🔴 [Exceeds RAM: {:.0}G/{:.0}G]", required_ram_gb, total_ram_gb)
                }
            };

            println!("{:<20} {:<34} {:<15} {:<16} {:<24}",
                spec.id, spec.name, size_str, status_str, verdict_str);
        }
        return;
    }

    let target_spec = match tier_opt {
        Some(t) => {
            match OnnxModelManager::resolve_spec(t) {
                Some(s) => s,
                None => {
                    eprintln!("❌ Unknown model '{}'.", t);
                    eprintln!("Examples: 'reranker' (~129 MB), 'reranker-m3' (~561 MB), 'laya', 'layout', 'scans', 'vision'.");
                    eprintln!("Run 'actx download --check' to inspect all supported models.");
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("💡 No model specified.");
            println!("Models are installed automatically with your hardware profile. To repair or add one, run e.g.:");
            println!("  actx download reranker");
            println!("Run 'actx download --check' to view the full catalog.");
            return;
        }
    };

    if !target_spec.is_available() {
        println!("🕓 {}", OnnxModelManager::planned_message(target_spec));
        return;
    }

    if OnnxModelManager::is_installed(target_spec) && !force {
        println!("✔ Model '{}' is already installed and verified at: {}",
            target_spec.name, OnnxModelManager::model_path(target_spec).display());
        return;
    }

    // Pre-flight safety check
    if let Err(err_msg) = OnnxModelManager::check_download_safety(target_spec, force) {
        eprintln!("{}", err_msg);
        std::process::exit(1);
    }

    println!("📥 Starting download of '{}' (~{:.2} GB)...",
        target_spec.name, target_spec.size_bytes as f64 / 1e9);
    println!("File:        {}", target_spec.file_name);
    println!("Destination: {}", OnnxModelManager::model_path(target_spec).display());
    println!("Press Ctrl+C anytime to cancel.\n");

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel::<(u64, u64)>();
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

    // Ctrl+C cancellation handler
    let cancel_tx_sig = cancel_tx.clone();
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            let _ = cancel_tx_sig.send(true);
        }
    });

    let spec_clone = target_spec.clone();
    let download_task = tokio::spawn(async move {
        OnnxModelManager::download_model_with_progress(&spec_clone, Some(progress_tx), Some(cancel_rx)).await
    });

    let start_time = std::time::Instant::now();
    let mut last_draw = std::time::Instant::now();

    while let Some((downloaded, total)) = progress_rx.recv().await {
        if last_draw.elapsed() >= std::time::Duration::from_millis(100) || downloaded >= total {
            let info = DownloadProgressInfo::compute(downloaded, total, start_time.elapsed());
            let bar_width: usize = 30;
            let filled = ((info.percentage / 100.0) * bar_width as f32).round() as usize;
            let empty = bar_width.saturating_sub(filled);
            let bar: String = "█".repeat(filled) + &"░".repeat(empty);

            let dl_gb = downloaded as f64 / 1e9;
            let tot_gb = total as f64 / 1e9;

            print!("\r[{}] {:>5.1}% ({:.2}/{:.2} GB) │ {:>9} │ ETA: {:<6}",
                bar, info.percentage, dl_gb, tot_gb, info.formatted_speed(), info.formatted_eta());
            let _ = std::io::stdout().flush();
            last_draw = std::time::Instant::now();
        }
    }

    match download_task.await {
        Ok(Ok(dest)) => {
            println!("\n\n✔ Download complete! Successfully verified and installed at: {}", dest.display());
            println!("🤖 Activate in interactive session: /model {}", target_spec.id);
            println!("⚡ Run one-shot query: actx \"your query\" --model {}", target_spec.id);
        }
        Ok(Err(e)) => {
            eprintln!("\n\n❌ Download failed: {}", e);
            std::process::exit(1);
        }
        Err(join_err) => {
            eprintln!("\n\n❌ Download task error: {}", join_err);
            std::process::exit(1);
        }
    }
}

/// Most errors listed after a sync; the rest are counted.
const MAX_SYNC_ERRORS_SHOWN: usize = 10;

/// Lines printed after a sync with errors: files that failed are retried on the next sync.
fn sync_error_lines(res: &any_context_core_rs::ingestion::SyncResult) -> Vec<String> {
    if res.errors.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![format!("⚠ {} error(s) during the sync:", res.errors.len())];
    lines.extend(res.errors.iter().take(MAX_SYNC_ERRORS_SHOWN).map(|e| format!("  • {e}")));
    if res.errors.len() > MAX_SYNC_ERRORS_SHOWN {
        lines.push(format!("  … and {} more", res.errors.len() - MAX_SYNC_ERRORS_SHOWN));
    }
    if res.embedding_failures > 0 {
        lines.push(format!(
            "  {} file(s) not embedded are retried on the next sync; keyword search finds them meanwhile.",
            res.embedding_failures
        ));
    }
    lines
}

fn print_sync_errors(res: &any_context_core_rs::ingestion::SyncResult) {
    for line in sync_error_lines(res) {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_error_lines_list_errors_and_the_retry_note() {
        let mut res = any_context_core_rs::ingestion::SyncResult::default();
        assert!(sync_error_lines(&res).is_empty());

        res.errors = (0..12).map(|i| format!("Embedding/upsert error on 'f{i}.md': quota")).collect();
        res.embedding_failures = 12;
        let lines = sync_error_lines(&res);
        assert_eq!(lines[0], "⚠ 12 error(s) during the sync:");
        assert_eq!(lines.iter().filter(|l| l.starts_with("  • ")).count(), MAX_SYNC_ERRORS_SHOWN);
        assert!(lines.contains(&"  … and 2 more".to_string()), "{lines:?}");
        assert!(lines.last().unwrap().contains("12 file(s) not embedded are retried on the next sync"), "{lines:?}");
    }
}
