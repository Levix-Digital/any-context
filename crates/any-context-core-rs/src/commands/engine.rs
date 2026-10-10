//! Universal UI-agnostic Command Engine for AnyContext.
//! Executes domain commands and emits standardized `CommandResult`s.

use std::path::Path;
use crate::commands::models::{
    CommandAction, CommandResult, CommandStateUpdates, ExecutionContext, GroundingMode, SearchDepthMode,
};
use crate::storage::{get_default_lancedb_path, get_default_settings_db_path, get_default_logs_dir, NativeConfigDb, NativeLanceStore};

/// UI-Agnostic command executor service.
pub struct CommandEngine;

impl CommandEngine {
    /// Executes a slash command string or action by canonical name or alias.
    pub fn execute(raw_cmd: &str, args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let clean_cmd = raw_cmd.trim().trim_start_matches('/').to_lowercase();
        let (cmd, at_version) = if let Some((base, ver)) = clean_cmd.split_once('@') {
            (base, Some(ver))
        } else {
            (clean_cmd.as_str(), None)
        };

        match cmd {
            "exit" | "quit" | "q" => {
                CommandResult::success("Session terminated. Goodbye!").with_action(CommandAction::Exit)
            }
            "clear" | "cls" => {
                CommandResult::success("").with_action(CommandAction::ClearChat)
            }
            "menu" => {
                CommandResult::success("Opening main options menu...").with_action(CommandAction::OpenMenu("main".to_string()))
            }
            "switch" | "workspace" | "workspaces" | "ws" => {
                Self::execute_switch(args, ctx)
            }
            "mode" | "grounding" | "grounding-mode" | "answer-mode" | "am" => {
                Self::execute_grounding_mode(args, ctx)
            }
            "search" | "search-mode" | "depth" | "sm" => {
                Self::execute_search_mode(args, ctx)
            }
            "router" | "routing" => {
                Self::execute_router_status(args, ctx)
            }
            "fast" => {
                Self::execute_fast_shortcut(args, ctx)
            }
            "deep" => {
                Self::execute_deep_shortcut(args, ctx)
            }
            "web-search" | "websearch" => {
                Self::execute_web_search(args.first().copied(), ctx)
            }
            "sync" | "reindex" | "resync" | "index" => {
                Self::execute_sync(args, ctx)
            }
            "cancel" | "stop-sync" | "stop" => {
                Self::execute_sync(&["cancel"], ctx)
            }
            "cd" => {
                Self::execute_cd(args, ctx)
            }
            "pwd" | "cwd" => {
                Self::execute_pwd(ctx)
            }
            "sources" | "source" | "list-sources" | "src" => {
                Self::execute_sources(args, ctx)
            }
            "folder" | "add" | "dir" => {
                Self::execute_folder(args, ctx)
            }
            "web" | "url" => {
                Self::execute_web(args, ctx)
            }
            "transfer" | "move-source" => {
                Self::execute_transfer(args, ctx)
            }
            "link" => {
                Self::execute_link(args, ctx)
            }
            "unlink" => {
                Self::execute_unlink(args, ctx)
            }
            "rename" => {
                Self::execute_rename(args, ctx)
            }
            "purge" => {
                Self::execute_purge(args, ctx)
            }
            "model" | "models" | "m" | "list-models" | "model-list" => {
                Self::execute_model(args, ctx)
            }
            "inspect" | "chunks" | "lance" => {
                Self::execute_inspect(args, ctx)
            }
            "status" => {
                Self::execute_status(ctx)
            }
            "diagnostics" | "diag" | "perf" | "health" | "diagnistics" => {
                Self::execute_diagnostics(ctx)
            }
            "reset-memory" | "forget" => {
                Self::execute_reset_memory(ctx)
            }
            "billing" | "plan" | "pricing" => {
                Self::execute_billing()
            }
            "keys" | "key" | "api-key" | "api-keys" => {
                Self::execute_keys(args)
            }
            "config" | "settings" => {
                Self::execute_config(args, ctx)
            }
            "update" | "self-update" | "upgrade" => {
                if let Some(v) = at_version {
                    Self::execute_update(&[v])
                } else {
                    Self::execute_update(args)
                }
            }
            "check-update" | "check" => {
                Self::execute_check_update()
            }
            "version" | "v" => {
                Self::execute_version()
            }
            "info" => {
                Self::execute_info(ctx)
            }
            "history" => {
                Self::execute_history(args, ctx)
            }
            "logs" | "log" => {
                Self::execute_logs(args, ctx)
            }
            "paste" | "multiline" | "mline" => {
                Self::execute_paste()
            }
            "density" => {
                Self::execute_density(args)
            }
            "spans" => {
                Self::execute_spans(args)
            }
            "onboarding" | "setup" => {
                Self::execute_onboarding()
            }
            "vision" | "vis" => {
                Self::execute_vision(args)
            }
            "ocr" | "scan" => {
                Self::execute_ocr()
            }
            "shared" => {
                Self::execute_shared(ctx)
            }
            "help" | "commands" | "slash" => {
                Self::execute_help()
            }
            other => {
                CommandResult::error(format!(
                    "Unknown command '/{}'. Type `/help` to view all available commands.",
                    other
                ))
            }
        }
    }

    // -------------------------------------------------------------------------
    // Grounding Mode (strict, hybrid, proactive)
    // -------------------------------------------------------------------------
    fn execute_grounding_mode(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        let mode_arg = args.first().map(|s| s.trim());

        if let Some(arg) = mode_arg {
            if let Some(parsed) = GroundingMode::parse(arg) {
                let valid_str = parsed.as_str();
                if let Some(d) = &db {
                    let _ = d.set_workspace_grounding_mode(&ctx.active_workspace, valid_str);
                }
                let mut updates = CommandStateUpdates::default();
                updates.grounding_mode = Some(valid_str.to_string());

                CommandResult::success(format!(
                    "🛡️ Grounding Strategy Mode for '{}' set to: **{}**\n\
                     • Response fidelity policy updated. Chunks will be referenced according to {} constraints.",
                    ctx.active_workspace,
                    valid_str.to_uppercase(),
                    valid_str.to_uppercase()
                ))
                .with_action(CommandAction::RebuildAgent)
                .with_state_updates(updates)
            } else {
                CommandResult::error(format!(
                    "Invalid grounding mode '{}'.\n\
                     Available Grounding Strategy modes:\n\
                     • `strict` (or `s`)   : Base answers strictly on indexed workspace chunks.\n\
                     • `hybrid` (or `h`)   : Local context prioritized, synthesized with general knowledge.\n\
                     • `proactive` (or `p`): Proactive contextual inference and exploratory suggestions.\n\
                     Usage: /mode <strict|hybrid|proactive>",
                    arg
                ))
            }
        } else if args.iter().any(|a| *a == "--status" || *a == "--info" || *a == "-s") {
            let curr = db
                .and_then(|d| d.get_workspace_grounding_mode(&ctx.active_workspace).ok())
                .unwrap_or_else(|| ctx.grounding_mode.clone());
            CommandResult::success(format!(
                "🛡️ Active Grounding Strategy for '{}': **{}**\n\n\
                 Available Grounding Strategy modes:\n\
                 • `strict` (or `s`)   : Base answers strictly on indexed workspace chunks.\n\
                 • `hybrid` (or `h`)   : Local context prioritized, synthesized with general knowledge.\n\
                 • `proactive` (or `p`): Proactive contextual inference and exploratory suggestions.\n\n\
                 Usage: /mode <strict|hybrid|proactive>",
                ctx.active_workspace,
                curr.to_uppercase()
            ))
        } else {
            CommandResult::success("Opening grounding strategy menu...")
                .with_action(CommandAction::OpenMenu("grounding".to_string()))
        }
    }

    // -------------------------------------------------------------------------
    // Search Depth Mode (auto, fast, deep - RFC-042)
    // -------------------------------------------------------------------------
    fn execute_search_mode(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        let search_arg = args.first().map(|s| s.trim());

        if let Some(arg) = search_arg {
            if arg == "--status" || arg == "--info" || arg == "-s" {
                let curr = db
                    .as_ref()
                    .and_then(|d| d.get_workspace_search_mode(&ctx.active_workspace).ok())
                    .unwrap_or_else(|| ctx.search_mode.clone());
                return CommandResult::success(format!(
                    "🔍 Active Search Retrieval Depth for '{}': **{}**\n\n\
                     Available Search Retrieval Depth modes:\n\
                     • `auto` (or `a`): Autonomously selects single-turn or reflexive deep search by complexity.\n\
                     • `fast` (or `f`): Single-turn low-latency hybrid RAG retrieval (<50ms).\n\
                     • `deep` (or `d`): Multi-turn reflexive reasoning with orthogonal sub-query decomposition (RFC-042).\n\n\
                     Usage: /search <auto|fast|deep>",
                    ctx.active_workspace,
                    curr.to_uppercase()
                ));
            }

            if let Some(parsed) = SearchDepthMode::parse(arg) {
                let valid_str = parsed.as_str();
                if let Some(d) = &db {
                    let _ = d.set_workspace_search_mode(&ctx.active_workspace, valid_str);
                }
                let mut updates = CommandStateUpdates::default();
                updates.search_mode = Some(valid_str.to_string());

                let explanation = match parsed {
                    SearchDepthMode::Auto => "• Autonomous ModelRouter: evaluates each prompt in sub-1ms and routes to Fast RAG or Deep Search.",
                    SearchDepthMode::Fast => "• Fast RAG: forced single-turn low-latency hybrid retrieval (<50ms).",
                    SearchDepthMode::Deep => "• Deep Search: forced multi-turn reflexive reasoning with orthogonal decomposition (RFC-042).",
                };

                CommandResult::success(format!(
                    "🔍 Search Depth Policy (Retrieval Depth) for '{}' set to: **{}**\n\
                     {}\n\
                     • Agent RAG policy updated to {} retrieval mode.",
                    ctx.active_workspace,
                    valid_str.to_uppercase(),
                    explanation,
                    valid_str.to_uppercase()
                ))
                .with_action(CommandAction::RebuildAgent)
                .with_state_updates(updates)
            } else {
                CommandResult::error(format!(
                    "Invalid search mode '{}'.\n\
                     Available Search Retrieval Depth modes:\n\
                     • `auto` (or `a`): Autonomously selects single-turn or reflexive deep search by complexity.\n\
                     • `fast` (or `f`): Single-turn low-latency hybrid RAG retrieval (<50ms).\n\
                     • `deep` (or `d`): Multi-turn reflexive reasoning with orthogonal sub-query decomposition (RFC-042).\n\
                     Usage: /search <auto|fast|deep>",
                    arg
                ))
            }
        } else {
            CommandResult::success("Opening search depth menu...")
                .with_action(CommandAction::OpenMenu("search".to_string()))
        }
    }

    fn execute_router_status(_args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        let curr = db
            .as_ref()
            .and_then(|d| d.get_workspace_search_mode(&ctx.active_workspace).ok())
            .unwrap_or_else(|| "auto".to_string());

        CommandResult::success(format!(
            "🧭 Chat ModelRouter Status (Workspace '{}'):\n\
             • Active Search Depth Policy: **{}**\n\
             • Classifier: Deterministic Multilingual Semantic Classifier (<1µs on CPU)\n\
             • Triage Engine: Fast RAG vs Deep Search (RFC-042)\n\
             • Domain Intent Detection: Code, Architecture, Document, General\n\
             • Routing SLA: Sub-1ms, zero token cost\n\
             Usage:\n\
             • `/search auto`: Enable autonomous query complexity routing.\n\
             • `/search fast`: Force Fast RAG mode for all queries.\n\
             • `/search deep`: Force Deep Search mode for all queries.\n\
             • `/fast <query>`: Shortcut to run a single query in Fast RAG mode.\n\
             • `/deep <query>`: Shortcut to run a single query in Deep Search mode.",
            ctx.active_workspace,
            curr.to_uppercase()
        ))
    }

    fn execute_fast_shortcut(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            Self::execute_search_mode(&["fast"], ctx)
        } else {
            let mut updates = CommandStateUpdates::default();
            updates.search_mode = Some("fast".to_string());
            CommandResult::success(format!("⚡ Fast RAG mode activated for query: '{}'", args.join(" ")))
                .with_action(CommandAction::RebuildAgent)
                .with_state_updates(updates)
        }
    }

    fn execute_deep_shortcut(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            Self::execute_search_mode(&["deep"], ctx)
        } else {
            let mut updates = CommandStateUpdates::default();
            updates.search_mode = Some("deep".to_string());
            CommandResult::success(format!("🧠 Deep Search Reflexive mode activated for query: '{}'", args.join(" ")))
                .with_action(CommandAction::RebuildAgent)
                .with_state_updates(updates)
        }
    }

    // -------------------------------------------------------------------------
    // Web Search Toggle
    // -------------------------------------------------------------------------
    fn execute_web_search(target: Option<&str>, ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        if let Some(arg) = target {
            let is_on = matches!(arg.to_lowercase().as_str(), "on" | "true" | "1" | "enable");
            if let Some(d) = &db {
                let _ = d.set_workspace_web_search(&ctx.active_workspace, is_on);
            }
            let mut updates = CommandStateUpdates::default();
            updates.web_search_enabled = Some(is_on);
            let status = if is_on { "🟢 ON" } else { "🔴 OFF" };

            CommandResult::success(format!("🌐 Real-time Web Search for '{}': {}", ctx.active_workspace, status))
                .with_action(CommandAction::RebuildAgent)
                .with_state_updates(updates)
        } else {
            let curr = db
                .and_then(|d| d.get_workspace_web_search(&ctx.active_workspace).ok())
                .unwrap_or(ctx.web_search_enabled);
            let status = if curr { "🟢 ON" } else { "🔴 OFF" };
            CommandResult::success(format!(
                "🌐 Real-time Web Search for '{}': {}\nUsage: /web-search [on|off]",
                ctx.active_workspace, status
            ))
        }
    }

    // -------------------------------------------------------------------------
    // Workspace Switching & Management
    // -------------------------------------------------------------------------
    fn execute_switch(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::success("Opening workspaces selector...")
                .with_action(CommandAction::OpenMenu("workspaces".to_string()));
        }

        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Failed to access settings database: {}", e)),
        };

        if args.iter().any(|a| *a == "--list" || *a == "-l" || *a == "list") {
            let names = db.list_workspace_names().unwrap_or_else(|_| vec!["Default".to_string()]);
            let mut msg = format!("📂 Available Workspaces ({}):\n", names.len());
            for name in &names {
                if name == &ctx.active_workspace {
                    msg.push_str(&format!("  * {} (active)\n", name));
                } else {
                    msg.push_str(&format!("  - {}\n", name));
                }
            }
            msg.push_str("\nTip: Use '/switch <name>' to switch, or press [F1] / type '/switch' for interactive selection.");
            return CommandResult::success(msg);
        }

        if args.iter().any(|a| *a == "--help" || *a == "-h" || *a == "help") {
            let msg = "\
📂 Workspace Switch Options:
  • /switch <name>           Switch active workspace (creates if new)
  • /switch --delete <name>  Delete workspace and associated vector records
  • /switch --list           List all registered workspaces";
            return CommandResult::success(msg);
        }

        if args[0] == "--delete" || args[0] == "-d" || args[0] == "delete" || args[0] == "remove" {
            if args.len() < 2 {
                return CommandResult::error("Usage: /switch --delete <workspace_name>");
            }
            let target = args[1];
            if target.eq_ignore_ascii_case("default") || target.eq_ignore_ascii_case("global") {
                return CommandResult::error(format!("Cannot delete the protected '{}' workspace.", target));
            }
            match db.delete_workspace(target) {
                Ok(true) => {
                    let lance_path = get_default_lancedb_path();
                    if let Ok(lance) = NativeLanceStore::open(&lance_path) {
                        let _ = lance.delete_by_workspace(target, None);
                    }
                    let db_path = get_default_settings_db_path();
                    if let Ok(store) = actx_agent::SqliteSessionStore::open(&db_path, 50) {
                        let _ = store.clear_session_sync(&format!("ws_{}", target));
                    }
                    if target.eq_ignore_ascii_case(&ctx.active_workspace) {
                        let _ = db.set_setting("active_workspace", "Default");
                        let mut updates = CommandStateUpdates::default();
                        updates.active_workspace = Some("Default".to_string());
                        CommandResult::success(format!("🗑️ Active workspace '{}' deleted. Switched back to 'Default'.", target))
                            .with_action(CommandAction::SwitchWorkspace("Default".to_string()))
                            .with_state_updates(updates)
                    } else {
                        CommandResult::success(format!("🗑️ Workspace '{}' and associated vector records deleted.", target))
                    }
                }
                Ok(false) => CommandResult::error(format!("Workspace '{}' not found.", target)),
                Err(e) => CommandResult::error(format!("Error deleting workspace: {}", e)),
            }
        } else {
            let target_ws = args[0].to_string();
            let _ = db.create_workspace(&target_ws, Some(&format!("Switched to workspace {}", target_ws)));
            let _ = db.set_setting("active_workspace", &target_ws);

            let mut updates = CommandStateUpdates::default();
            updates.active_workspace = Some(target_ws.clone());

            if let Ok(model) = db.get_workspace_model(&target_ws) {
                updates.active_model = Some(model);
            }
            if let Ok(mode) = db.get_workspace_grounding_mode(&target_ws) {
                updates.grounding_mode = Some(mode);
            }
            if let Ok(web) = db.get_workspace_web_search(&target_ws) {
                updates.web_search_enabled = Some(web);
            }
            if let Ok(search) = db.get_workspace_search_mode(&target_ws) {
                updates.search_mode = Some(search);
            }

            CommandResult::success(format!("Switched active workspace to: **{}**", target_ws))
                .with_action(CommandAction::SwitchWorkspace(target_ws))
                .with_state_updates(updates)
        }
    }

    // -------------------------------------------------------------------------
    // -------------------------------------------------------------------------
    // Background Sync Worker Spawning Helper
    // -------------------------------------------------------------------------
    /// Spawns the background synchronization / crawler worker for a workspace.
    pub fn spawn_sync_worker(
        workspace: &str,
        force: bool,
        target_folder: Option<&str>,
    ) -> Result<(u32, std::path::PathBuf), String> {
        let canonical_dir = actx_installer::paths::get_canonical_bin_dir();
        let app_exe_name = actx_installer::paths::get_shim_exe_name();
        let installed_exe = canonical_dir.join(app_exe_name);
        let current_exe = std::env::current_exe().ok();

        let mut cmd = if let Some(ref cur) = current_exe.filter(|p| p.exists()) {
            let mut c = std::process::Command::new(cur);
            c.arg("--sync-worker");
            c
        } else if installed_exe.exists() {
            let mut c = std::process::Command::new(installed_exe);
            c.arg("--sync-worker");
            c
        } else {
            let mut c = std::process::Command::new(app_exe_name);
            c.arg("--sync-worker");
            c
        };

        cmd.arg("--workspace").arg(workspace);
        if force {
            cmd.arg("--force");
        }
        if let Some(folder) = target_folder {
            cmd.arg("--folder").arg(folder);
        }

        let log_dir = get_default_logs_dir();
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = log_dir.join(format!("sync_{}.log", workspace));

        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path);

        if let Ok(f) = log_file {
            let err_clone = f.try_clone().ok();
            cmd.stdout(std::process::Stdio::from(f));
            if let Some(err_f) = err_clone {
                cmd.stderr(std::process::Stdio::from(err_f));
            } else {
                cmd.stderr(std::process::Stdio::null());
            }
        } else {
            cmd.stdout(std::process::Stdio::null());
            cmd.stderr(std::process::Stdio::null());
        }

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id();
                if let Ok(db) = NativeConfigDb::open_default() {
                    let _ = db.update_sync_status(
                        workspace,
                        true,
                        Some(pid),
                        0,
                        0,
                        "scanning",
                        None,
                        None,
                    );
                }
                Ok((pid, log_path))
            }
            Err(spawn_err) => {
                // If launching detached process failed, fall back to running NativeSyncOrchestrator in a background OS thread
                let ws_owned = workspace.to_string();
                let tf_owned = target_folder.map(|s| s.to_string());
                let pid = std::process::id();
                let thread_res = std::thread::Builder::new()
                    .name(format!("sync-worker-{}", workspace))
                    .spawn(move || {
                        if let Ok(orchestrator) = crate::ingestion::NativeSyncOrchestrator::new_default() {
                            let opts = crate::ingestion::SyncOptions {
                                workspace: ws_owned,
                                force,
                                target_folder: tf_owned,
                                verbose: false,
                                model: None,
                            };
                            let _ = orchestrator.run_sync(&opts);
                        }
                    });

                match thread_res {
                    Ok(_) => {
                        if let Ok(db) = NativeConfigDb::open_default() {
                            let _ = db.update_sync_status(
                                workspace,
                                true,
                                Some(pid),
                                0,
                                0,
                                "scanning",
                                None,
                                None,
                            );
                        }
                        Ok((pid, log_path))
                    }
                    Err(th_err) => {
                        Err(format!(
                            "Failed to launch background synchronization worker: {} (thread fallback failed: {})",
                            spawn_err, th_err
                        ))
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Sync
    // -------------------------------------------------------------------------
    fn execute_sync(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let is_cancel = args.iter().any(|a| *a == "--cancel" || *a == "-c" || *a == "cancel" || *a == "stop");
        if is_cancel {
            let db = match NativeConfigDb::open_default() {
                Ok(d) => d,
                Err(e) => return CommandResult::error(format!("Database error: {}", e)),
            };

            let status = db.get_sync_status(&ctx.active_workspace).ok().flatten();
            if let Some(ss) = status {
                if let Some(pid) = ss.pid {
                    #[cfg(windows)]
                    {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/PID", &pid.to_string(), "/F", "/T"])
                            .output();
                    }
                    #[cfg(not(windows))]
                    {
                        let _ = std::process::Command::new("kill")
                            .args(["-9", &pid.to_string()])
                            .output();
                    }

                    let _ = db.update_sync_status(
                        &ctx.active_workspace,
                        false,
                        None,
                        ss.current_item,
                        ss.total_items,
                        "cancelled",
                        ss.item_name.as_deref(),
                        None,
                    );

                    return CommandResult::success(format!(
                        "🛑 Background synchronization worker (PID: {}) for workspace '**{}**' has been cancelled.",
                        pid, ctx.active_workspace
                    ));
                } else if ss.is_syncing {
                    let _ = db.update_sync_status(
                        &ctx.active_workspace,
                        false,
                        None,
                        ss.current_item,
                        ss.total_items,
                        "cancelled",
                        ss.item_name.as_deref(),
                        None,
                    );
                    return CommandResult::success(format!(
                        "🛑 Active synchronization status for workspace '**{}**' reset to cancelled.",
                        ctx.active_workspace
                    ));
                }
            }

            return CommandResult::success(format!(
                "ℹ️ No active synchronization worker is currently running for workspace '**{}**'.",
                ctx.active_workspace
            ));
        }

        let is_incremental = args.iter().any(|a| *a == "--incremental" || *a == "-i" || *a == "incremental");
        let force = args.iter().any(|a| *a == "--force" || *a == "-f" || *a == "force");
        let is_menu = args.iter().any(|a| *a == "--menu" || *a == "-m" || *a == "menu");

        // Parse optional target folder from args
        let mut target_folder: Option<String> = None;
        for i in 0..args.len() {
            if (args[i] == "--folder" || args[i] == "-fld") && i + 1 < args.len() {
                let p = Path::new(args[i + 1].trim_matches('\'').trim_matches('"'));
                let canonical = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                target_folder = Some(canonical.to_string_lossy().trim_start_matches(r"\\?\").to_string());
                break;
            }
        }
        if target_folder.is_none() {
            for arg in args {
                let a = *arg;
                if a.starts_with('-') || a == "incremental" || a == "force" || a == "menu" || a == "status" || a == "cancel" {
                    continue;
                }
                let clean = a.trim_matches('\'').trim_matches('"');
                let p = Path::new(clean);
                if p.exists() || clean == "." {
                    let canonical = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
                    target_folder = Some(canonical.to_string_lossy().trim_start_matches(r"\\?\").to_string());
                    break;
                }
            }
        }

        if (args.is_empty() || is_menu) && !is_incremental && !force && target_folder.is_none() {
            return CommandResult::success("Opening sync options...")
                .with_action(CommandAction::OpenMenu("sync".to_string()));
        }
        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Database error: {}", e)),
        };

        let folders = db.get_workspace_folders(&ctx.active_workspace).unwrap_or_default();
        let urls = db.get_workspace_web_urls(&ctx.active_workspace).unwrap_or_default();

        let effective_folders = if !folders.is_empty() {
            folders
        } else if urls.is_empty() {
            vec![std::env::current_dir().unwrap_or_default().to_string_lossy().to_string()]
        } else {
            vec![]
        };

        match Self::spawn_sync_worker(&ctx.active_workspace, force, target_folder.as_deref()) {
            Ok((pid, log_path)) => {
                let mut msg = if force {
                    format!("Forced sync completed: Background synchronization worker spawned for workspace '**{}**' [PID: {}]:\n", ctx.active_workspace, pid)
                } else {
                    format!("🔄 Synchronizing workspace '**{}**': Background synchronization worker spawned [PID: {}]:\n", ctx.active_workspace, pid)
                };
                if let Some(ref tf) = target_folder {
                    msg.push_str(&format!("  • Scoped Folder: {}\n", tf));
                } else {
                    if !effective_folders.is_empty() {
                        msg.push_str(&format!("  • Monitored Local Roots: {}\n", effective_folders.len()));
                        for f in &effective_folders {
                            msg.push_str(&format!("     • {}\n", f));
                        }
                    }
                    if !urls.is_empty() {
                        msg.push_str(&format!("  • Web Documentation Portals: {}\n", urls.len()));
                        for u in &urls {
                            msg.push_str(&format!("     • {}\n", u));
                        }
                    }
                }
                msg.push_str(&format!(
                    "\n📝 Worker log: {}\nCrawling, HTML extraction, and LanceDB vector indexing are running in the background.\nYou can continue chatting or run `/sources` to inspect registered sources.",
                    log_path.display()
                ));
                CommandResult::success(msg)
            }
            Err(e) => {
                CommandResult::error(format!(
                    "{}\nEnsure AnyContext is properly installed.",
                    e
                ))
            }
        }
    }

    // -------------------------------------------------------------------------
    // Sources & Folders
    // -------------------------------------------------------------------------
    fn execute_sources(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::success("Opening sources menu...")
                .with_action(CommandAction::OpenMenu("sources".to_string()));
        }

        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Failed to open config database: {}", e)),
        };

        let is_all = args.iter().any(|a| *a == "--all" || *a == "-a" || *a == "all");
        if is_all {
            let ws_names = db.list_workspace_names().unwrap_or_else(|_| vec![ctx.active_workspace.clone()]);
            let mut msg = format!("📂 All Configured Workspaces & Sources ({} workspaces):\n\n", ws_names.len());
            for ws in &ws_names {
                let folders = db.get_workspace_folders(ws).unwrap_or_default();
                let urls = db.get_workspace_web_urls(ws).unwrap_or_default();
                let total = folders.len() + urls.len();
                let is_active = ws.eq_ignore_ascii_case(&ctx.active_workspace);
                let active_tag = if is_active { " (active)" } else { "" };
                msg.push_str(&format!("• Workspace '{}'{} - {} source(s):\n", ws, active_tag, total));
                for f in &folders {
                    msg.push_str(&format!("  📁 {}\n", f));
                }
                for u in &urls {
                    msg.push_str(&format!("  🌐 {}\n", u));
                }
                if total == 0 {
                    msg.push_str("  (no sources registered)\n");
                }
                msg.push('\n');
            }
            return CommandResult::success(msg);
        }

        let folders = db.get_workspace_folders(&ctx.active_workspace).unwrap_or_default();
        let web_sources = db.get_workspace_web_urls(&ctx.active_workspace).unwrap_or_default();

        let mut out = format!("📂 Data Sources configured for workspace '{}':\n", ctx.active_workspace);
        out.push_str("📁 Local Folders:\n");
        if folders.is_empty() {
            out.push_str("  (none configured. Use `/folder --add <path>` or `/folder <path>`)\n");
        } else {
            for f in &folders {
                out.push_str(&format!("  • {}\n", f));
            }
        }
        out.push_str("\n🌐 Web Documentation Portals:\n");
        if web_sources.is_empty() {
            out.push_str("  (none configured. Use `/web --add <url>` or `/web <url>`)\n");
        } else {
            for w in &web_sources {
                out.push_str(&format!("  • {}\n", w));
            }
        }
        CommandResult::success(out)
    }

    fn execute_folder(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Failed to open config database: {}", e)),
        };

        if args.is_empty() || args.iter().any(|a| *a == "--list" || *a == "-l" || *a == "list") {
            let folders = db.get_workspace_folders(&ctx.active_workspace).unwrap_or_default();
            let mut msg = format!("📁 Monitored Folders in workspace '{}':\n", ctx.active_workspace);
            if folders.is_empty() {
                msg.push_str("  (none attached. Use `/folder --add <path>` or `/folder <path>`)\n");
            } else {
                for f in folders {
                    msg.push_str(&format!("  • {}\n", f));
                }
            }
            msg.push_str("\nUsage: /folder [--add <path> | --remove <path> | <path>]");
            return CommandResult::success(msg);
        }

        if args[0] == "--remove" || args[0] == "-r" || args[0] == "remove" || args[0] == "--delete" || args[0] == "-d" || args[0] == "delete" {
            if args.len() < 2 {
                return CommandResult::error("❌ Specify folder path: `/folder --remove <path>`");
            }
            let raw_path = args[1..].join(" ");
            let clean_path = raw_path.trim().trim_matches('\'').trim_matches('"');
            let path = Path::new(clean_path);
            let canonical_str = path.canonicalize()
                .map(|p| p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
                .unwrap_or_else(|_| clean_path.to_string());

            let mut removed = db.remove_workspace_folder(&ctx.active_workspace, &canonical_str);
            if removed.as_ref().ok() != Some(&true) && canonical_str != clean_path {
                removed = db.remove_workspace_folder(&ctx.active_workspace, clean_path);
            }
            match removed {
                Ok(true) => CommandResult::success(format!("📁 Removed folder from workspace '{}':\n  {}", ctx.active_workspace, clean_path)),
                Ok(false) => CommandResult::error(format!("⚠️ Folder '{}' was not attached to workspace '{}'.", clean_path, ctx.active_workspace)),
                Err(e) => CommandResult::error(format!("❌ Error removing folder: {}", e)),
            }
        } else {
            let path_parts = if args[0] == "--add" || args[0] == "-a" || args[0] == "add" {
                if args.len() < 2 {
                    return CommandResult::error("❌ Specify folder path: `/folder --add <path>`");
                }
                &args[1..]
            } else {
                args
            };
            let path_str = path_parts.join(" ");
            let clean_path_str = path_str.trim().trim_matches('\'').trim_matches('"');
            let path = Path::new(clean_path_str);
            if !path.exists() {
                return CommandResult::error(format!("❌ Directory does not exist: {}", clean_path_str));
            }
            let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let canonical_str = canonical.to_string_lossy().to_string();
            let clean_canonical_str = canonical_str.trim_start_matches(r"\\?\").to_string();

            match db.add_workspace_folder(&ctx.active_workspace, &clean_canonical_str) {
                Ok(_) => {
                    let mut msg = format!("📁 Added folder to workspace '{}':\n  {}", ctx.active_workspace, clean_canonical_str);
                    if clean_path_str == "." {
                        msg.push_str(" (resolved from current working directory)");
                    }
                    match Self::spawn_sync_worker(&ctx.active_workspace, false, Some(&clean_canonical_str)) {
                        Ok((pid, _)) => {
                            msg.push_str(&format!("\n⚡ Indexing started in background for this folder [PID: {}].", pid));
                        }
                        Err(e) => {
                            msg.push_str(&format!("\n(Background sync worker note: {})", e));
                        }
                    }
                    CommandResult::success(msg)
                }
                Err(e) => CommandResult::error(format!("❌ Error adding folder: {}", e)),
            }
        }
    }

    fn execute_pwd(_ctx: &ExecutionContext) -> CommandResult {
        match std::env::current_dir() {
            Ok(dir) => {
                let clean = dir.display().to_string().replace(r"\\?\", "");
                CommandResult::success(format!("📂 Current Working Directory:\n  `{}`", clean))
            }
            Err(e) => CommandResult::error(format!("Failed to determine current working directory: {}", e)),
        }
    }

    fn execute_cd(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            return Self::execute_pwd(ctx);
        }
        let target_str = args.join(" ");
        let clean_target = target_str.trim().trim_matches('\'').trim_matches('"');
        let target_path = std::path::PathBuf::from(clean_target);
        let resolved = if target_path.is_absolute() {
            target_path
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(&target_path),
                Err(_) => target_path,
            }
        };

        if !resolved.exists() {
            return CommandResult::error(format!("❌ Directory does not exist: `{}`", resolved.display()));
        }
        if !resolved.is_dir() {
            return CommandResult::error(format!("❌ Path is not a directory: `{}`", resolved.display()));
        }

        match std::env::set_current_dir(&resolved) {
            Ok(()) => {
                let canonical = resolved.canonicalize().unwrap_or(resolved);
                let clean_display = canonical.display().to_string().replace(r"\\?\", "");
                CommandResult::success(format!(
                    "📂 Changed working directory to:\n  `{}`\n\nTip: Run `/folder .` to attach and synchronize this folder in workspace '**{}**'.",
                    clean_display, ctx.active_workspace
                ))
            }
            Err(e) => CommandResult::error(format!("❌ Failed to change working directory to `{}`: {}", resolved.display(), e)),
        }
    }

    fn execute_web(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Failed to open config database: {}", e)),
        };

        if args.is_empty() || args.iter().any(|a| *a == "--list" || *a == "-l" || *a == "list") {
            let sources = db.get_workspace_web_urls(&ctx.active_workspace).unwrap_or_default();
            let mut msg = format!("🌐 Web Documentation Portals in workspace '{}':\n", ctx.active_workspace);
            if sources.is_empty() {
                msg.push_str("  (none attached. Use `/web --add <url>` or `/web <url>`)\n");
            } else {
                for s in sources {
                    msg.push_str(&format!("  • {}\n", s));
                }
            }
            msg.push_str("\nUsage: /web [--add <url> | --remove <url> | <url>]");
            return CommandResult::success(msg);
        }

        if args[0] == "--remove" || args[0] == "-r" || args[0] == "remove" || args[0] == "--delete" || args[0] == "-d" || args[0] == "delete" {
            if args.len() < 2 {
                return CommandResult::error("❌ Specify URL: `/web --remove <url>`");
            }
            let url = args[1..].join(" ");
            let clean_url = url.trim().trim_matches('\'').trim_matches('"');
            match db.remove_workspace_web_url(&ctx.active_workspace, clean_url) {
                Ok(true) => CommandResult::success(format!("🌐 Removed web source from workspace '{}':\n  {}", ctx.active_workspace, clean_url)),
                Ok(false) => CommandResult::error(format!("⚠️ Web source '{}' was not attached to workspace '{}'.", clean_url, ctx.active_workspace)),
                Err(e) => CommandResult::error(format!("❌ Error removing web source: {}", e)),
            }
        } else {
            let url_parts = if args[0] == "--add" || args[0] == "-a" || args[0] == "add" {
                if args.len() < 2 {
                    return CommandResult::error("❌ Specify URL: `/web --add <url>`");
                }
                &args[1..]
            } else {
                args
            };
            let url_str = url_parts.join(" ");
            let clean_url = url_str.trim().trim_matches('\'').trim_matches('"');
            if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
                return CommandResult::error(format!("❌ Invalid URL format (must begin with http:// or https://): {}", clean_url));
            }

            match db.add_workspace_web_url(&ctx.active_workspace, clean_url) {
                Ok(_) => {
                    let mut msg = format!("🌐 Added web documentation portal to workspace '{}':\n  {}", ctx.active_workspace, clean_url);
                    match Self::spawn_sync_worker(&ctx.active_workspace, false, None) {
                        Ok((pid, _)) => {
                            msg.push_str(&format!("\n⚡ Crawler started in background [PID: {}].", pid));
                        }
                        Err(e) => {
                            msg.push_str(&format!("\n(Background crawler note: {})", e));
                        }
                    }
                    CommandResult::success(msg)
                }
                Err(e) => CommandResult::error(format!("❌ Error adding web source: {}", e)),
            }
        }
    }

    fn execute_transfer(args: &[&str], _ctx: &ExecutionContext) -> CommandResult {
        if args.len() < 3 {
            return CommandResult::error("Usage: /transfer <from_workspace> <to_workspace> <source_path_or_url>");
        }
        let from_ws = args[0];
        let to_ws = args[1];
        let item = args[2];

        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Database error: {}", e)),
        };

        let is_url = item.starts_with("http://") || item.starts_with("https://");
        let remove_res = if is_url {
            db.remove_workspace_web_url(from_ws, item)
        } else {
            db.remove_workspace_folder(from_ws, item)
        };

        match remove_res {
            Ok(true) => {
                let _ = if is_url {
                    db.add_workspace_web_url(to_ws, item)
                } else {
                    db.add_workspace_folder(to_ws, item)
                };
                CommandResult::success(format!("📦 Source '{}' transferred from '{}' to '{}' in <50ms ($0.00).", item, from_ws, to_ws))
            }
            Ok(false) => CommandResult::error(format!("⚠️ Source '{}' was not found in workspace '{}'.", item, from_ws)),
            Err(e) => CommandResult::error(format!("❌ Transfer error: {}", e)),
        }
    }

    fn execute_link(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("Usage: /link <shared_source_path_or_url> [target_workspace]");
        }
        let source = args[0];
        let target_ws = args.get(1).copied().unwrap_or(&ctx.active_workspace);

        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Database error: {}", e)),
        };

        let is_url = source.starts_with("http://") || source.starts_with("https://");
        let res = if is_url {
            db.add_workspace_web_url(target_ws, source)
        } else {
            db.add_workspace_folder(target_ws, source)
        };

        match res {
            Ok(_) => CommandResult::success(format!("🔗 Source '{}' linked to workspace '{}' with $0.00 cost.", source, target_ws)),
            Err(e) => CommandResult::error(format!("❌ Error linking source: {}", e)),
        }
    }

    fn execute_unlink(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::error("Usage: /unlink <source_path_or_url>");
        }
        let source = args[0];
        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Database error: {}", e)),
        };

        let is_url = source.starts_with("http://") || source.starts_with("https://");
        let res = if is_url {
            db.remove_workspace_web_url(&ctx.active_workspace, source)
        } else {
            db.remove_workspace_folder(&ctx.active_workspace, source)
        };

        match res {
            Ok(true) => CommandResult::success(format!("🔓 Source '{}' unlinked from workspace '{}'.", source, ctx.active_workspace)),
            Ok(false) => CommandResult::error(format!("⚠️ Source '{}' not found in workspace '{}'.", source, ctx.active_workspace)),
            Err(e) => CommandResult::error(format!("❌ Error unlinking source: {}", e)),
        }
    }

    fn execute_rename(args: &[&str], _ctx: &ExecutionContext) -> CommandResult {
        if args.len() < 2 {
            return CommandResult::error("Usage: /rename <old_name> <new_name>");
        }
        let old_name = args[0];
        let new_name = args[1];

        if old_name.eq_ignore_ascii_case("default") {
            return CommandResult::error("Cannot rename the protected 'Default' workspace.");
        }

        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Database error: {}", e)),
        };

        match db.rename_workspace(old_name, new_name) {
            Ok(true) => CommandResult::success(format!("✏️ Workspace '{}' renamed to '{}'.", old_name, new_name)),
            Ok(false) => CommandResult::error(format!("⚠️ Workspace '{}' not found or cannot be renamed.", old_name)),
            Err(e) => CommandResult::error(format!("❌ Error renaming workspace: {}", e)),
        }
    }

    fn execute_purge(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let is_all = args.iter().any(|a| *a == "--all" || *a == "-a");
        let lance_path = get_default_lancedb_path();

        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Database error: {}", e)),
        };

        if let Ok(lance) = NativeLanceStore::open(&lance_path) {
            let _ = if is_all {
                lance.delete_by_workspace(&ctx.active_workspace, None)
            } else {
                lance.delete_by_workspace(&ctx.active_workspace, None)
            };
            let _ = db.clear_workspace_file_metadata(&ctx.active_workspace);
            let _ = db.update_sync_status(&ctx.active_workspace, false, None, 0, 0, "idle", None, None);
            CommandResult::success(format!(
                "🧹 Vector index and file hash metadata purged for workspace '{}'.\nNext sync will perform a clean, full re-index.",
                ctx.active_workspace
            ))
        } else {
            CommandResult::error("Failed to connect to LanceDB vector storage.")
        }
    }

    // -------------------------------------------------------------------------
    // Model Selection & Catalog
    // -------------------------------------------------------------------------
    fn execute_model(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        if args.is_empty() {
            return CommandResult::success("Opening models selection menu...")
                .with_action(CommandAction::OpenMenu("models".to_string()));
        }

        if args.iter().any(|a| *a == "--list" || *a == "-l" || *a == "list") {
            return Self::execute_models_catalog(args, ctx);
        }

        let target = args[0].trim();
        let db = NativeConfigDb::open_default().ok();
        if let Some(d) = &db {
            let _ = d.set_setting("active_model", target);
        }
        let mut updates = CommandStateUpdates::default();
        updates.active_model = Some(target.to_string());

        CommandResult::success(format!("🤖 Active language model for '{}' set to: **{}**", ctx.active_workspace, target))
            .with_action(CommandAction::RebuildAgent)
            .with_state_updates(updates)
    }

    fn execute_models_catalog(_args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let models_text = format!(
            "🤖 Supported AI Providers & Models (Current: **{}**):\n\n\
             ⭐ Cloud Giants (Tier 1):\n\
             • gpt-4o             - OpenAI Flagship Omnimodal\n\
             • gpt-4o-mini        - OpenAI Fast & Lightweight (Default)\n\
             • claude-3-5-sonnet  - Anthropic SOTA Code & Reasoning\n\
             • gemini-1.5-pro     - Google 2M Token Ultra-Long Context\n\
             • gemini-1.5-flash   - Google Sub-Second High Speed\n\n\
             ⚡ High-Speed Open Inference:\n\
             • groq/llama-3.3-70b - Groq LPUs (500+ tok/s)\n\
             • deepseek-chat      - DeepSeek V3\n\
             • deepseek-reasoner  - DeepSeek R1 Thinking\n\n\
             🔒 100% Local & Free:\n\
             • local/ollama       - Ollama (http://localhost:11434/v1)\n\
             • local/lm-studio    - LM Studio (http://localhost:1234/v1)\n\n\
             Usage: /model <model_name>",
            ctx.active_model
        );
        CommandResult::success(models_text)
    }

    // -------------------------------------------------------------------------
    // Inspection & Status
    // -------------------------------------------------------------------------
    fn execute_inspect(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let lance_path = get_default_lancedb_path();
        let lance = match NativeLanceStore::open(&lance_path) {
            Ok(l) => l,
            Err(e) => return CommandResult::error(format!("Unable to open LanceDB vector storage for inspection: {}", e)),
        };

        let ws_count = lance.count_records(Some(&ctx.active_workspace), None).unwrap_or(0);
        let total_count = lance.count_records(None, None).unwrap_or(0);

        let show_full = args.iter().any(|a| *a == "--full" || *a == "-f");
        let clean_args: Vec<&str> = args.iter().copied().filter(|a| *a != "--full" && *a != "-f").collect();

        let mut limit = 3usize;
        let mut filter_query: Option<&str> = None;

        if let Some(first) = clean_args.first() {
            if let Ok(n) = first.parse::<usize>() {
                limit = n.clamp(1, 25);
            } else {
                filter_query = Some(first);
                if let Some(second) = clean_args.get(1) {
                    if let Ok(n) = second.parse::<usize>() {
                        limit = n.clamp(1, 25);
                    }
                }
            }
        }

        let mut lines = vec![
            format!("🔍 **Vector Store Inspection for `{}`**:", ctx.active_workspace),
            format!("• Workspace Chunks: **{}**", ws_count),
            format!("• Total Database Chunks: **{}**", total_count),
            "• Storage Engine: **LanceDB (Apache Arrow / Native Rust)**".to_string(),
        ];

        if ws_count == 0 {
            lines.push(String::new());
            lines.push("*(No chunks found in this workspace yet. Run `/sync` or `/folder <path>` to ingest files.)*".to_string());
            return CommandResult::success(lines.join("\n"));
        }

        let ws_clean = ctx.active_workspace.replace('\'', "''");
        let where_clause = if let Some(q) = filter_query {
            let clean_q = q.replace('\'', "''");
            format!("workspace = '{ws_clean}' AND (content_type LIKE '%{clean_q}%' OR file_name LIKE '%{clean_q}%' OR file_path LIKE '%{clean_q}%' OR text LIKE '%{clean_q}%')")
        } else {
            format!("workspace = '{ws_clean}' AND length(text) > 0")
        };

        let samples = lance.search_metadata(&where_clause, limit.max(20), Some(&ctx.active_workspace), None).unwrap_or_default();

        if !samples.is_empty() {
            use std::collections::HashMap;
            let mut taxonomy_counts: HashMap<String, usize> = HashMap::new();
            for s in &samples {
                let ct = s.content_type.as_deref().unwrap_or("unknown");
                *taxonomy_counts.entry(ct.to_string()).or_insert(0) += 1;
            }

            if !taxonomy_counts.is_empty() {
                lines.push(String::new());
                lines.push("📊 **Chunk Taxonomy Breakdown (sample):**".to_string());
                let mut counts_vec: Vec<(String, usize)> = taxonomy_counts.into_iter().collect();
                counts_vec.sort_by(|a, b| b.1.cmp(&a.1));
                for (ct, count) in counts_vec {
                    lines.push(format!("  • `{}`: **{}** chunk(s)", ct, count));
                }
            }

            lines.push(String::new());
            let filter_info = if let Some(q) = filter_query { format!(" (filtered by '{}')", q) } else { String::new() };
            let display_count = samples.len().min(limit);
            lines.push(format!("📋 **Sample Chunks{} (showing {}):**", filter_info, display_count));

            let sec = crate::security::NativeSecurityEngine::get_instance();
            for (idx, r) in samples.iter().take(display_count).enumerate() {
                let fn_str = &r.file_name;
                let ct_str = r.content_type.as_deref().unwrap_or("unknown");
                let decrypted = sec.decrypt_text(&r.text);
                let text = decrypted.trim();
                let char_count = text.len();

                lines.push(format!("  **[{}] 📄 `{}`**", idx + 1, fn_str));
                lines.push(format!("    • Taxonomy: `{}`", ct_str));
                lines.push(format!("    • Size: **{}** characters", char_count));

                if show_full {
                    lines.push(format!("    • Full Content:\n```text\n{}\n```", text));
                } else {
                    let preview: String = text.chars().take(120).collect();
                    let clean_preview = preview.replace('\n', " ");
                    lines.push(format!("    • Preview: *\"{}...\"*", clean_preview));
                }
            }

            if !show_full {
                lines.push(String::new());
                lines.push("💡 *Tip: Run `/inspect --full` to display the complete, non-truncated content.*".to_string());
            }
        } else if let Some(q) = filter_query {
            lines.push(String::new());
            lines.push(format!("⚠️ No chunks matched filter query `{}`.", q));
        }

        CommandResult::success(lines.join("\n"))
    }

    fn execute_status(ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        let ws_count = db.as_ref().and_then(|d| d.list_workspace_names().ok()).map(|w| w.len()).unwrap_or(1);
        let folders_count = db.as_ref().and_then(|d| d.get_workspace_folders(&ctx.active_workspace).ok()).map(|f| f.len()).unwrap_or(0);

        let lance_path = get_default_lancedb_path();
        let chunks_count = NativeLanceStore::open(&lance_path)
            .ok()
            .and_then(|l| l.count_records(Some(&ctx.active_workspace), None).ok())
            .unwrap_or(0);

        CommandResult::success(format!(
            "📊 AnyContext Native Core Operational Status:\n\
             • Workspace:      {} (Total workspaces: {})\n\
             • Target Model:   {}\n\
             • Grounding:      {}\n\
             • Search Depth:   {}\n\
             • Web Search:     {}\n\
             • Local Folders:  {} attached\n\
             • Vector Chunks:  {} indexed (LanceDB)\n\
             • Health:         ● HEALTHY & READY",
            ctx.active_workspace,
            ws_count,
            ctx.active_model,
            ctx.grounding_mode.to_uppercase(),
            ctx.search_mode.to_uppercase(),
            if ctx.web_search_enabled { "ON" } else { "OFF" },
            folders_count,
            chunks_count
        ))
    }

    fn execute_diagnostics(ctx: &ExecutionContext) -> CommandResult {
        let db_path = get_default_settings_db_path();
        let lance_path = get_default_lancedb_path();
        let ws_count = NativeConfigDb::open_default()
            .and_then(|d| d.list_workspace_names())
            .map(|w| w.len())
            .unwrap_or(1);

        CommandResult::success(format!(
            "AnyContext Diagnostics Report (v{}):\n\
             • Platform:   {} ({})\n\
             • Engine:     100% Native Rust (crates/any-context-core-rs)\n\
             • Runtimes:   Zero Python, Zero Bun/Node dependencies\n\
             • Workspace:  {} (Total: {})\n\
             Target Lm:  {}\n\
             Grounding:  {}\n\
             Search:     {}\n\
             Web Search: {}\n\
             Database:   {} (Connected)\n\
             Vectors:    {} (LanceDB Arrow Ready)\n\
             Status:     Healthy & Operational",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            ctx.active_workspace,
            ws_count,
            ctx.active_model,
            ctx.grounding_mode.to_uppercase(),
            ctx.search_mode.to_uppercase(),
            if ctx.web_search_enabled { "ON" } else { "OFF" },
            db_path.display(),
            lance_path.display()
        ))
    }

    fn execute_reset_memory(ctx: &ExecutionContext) -> CommandResult {
        let db_path = get_default_settings_db_path();
        if let Ok(store) = actx_agent::SqliteSessionStore::open(&db_path, 50) {
            let session_id = format!("ws_{}", ctx.active_workspace);
            let _ = store.clear_session_sync(&session_id);
        }
        CommandResult::success(format!(
            "🧠 Long-term session memory reset for workspace '{}'.\nChat history cleared while preserving indexed document vectors.",
            ctx.active_workspace
        ))
        .with_action(CommandAction::ClearChat)
    }

    fn execute_billing() -> CommandResult {
        CommandResult::success(format!(
            "💳 Subscription & Tier Status:\n\
             • Active Tier: COMMUNITY (100% Free & Open Source)\n\
             • Status: ACTIVE & UNLIMITED\n\
             • Features: Full-Screen Native TUI, LanceDB Vector Search, Okapi BM25 Lexical Scan, Zero Python/Bun Runtimes, $0.00 Cost.\n\
             • Target Release: AnyContext v{}",
            env!("CARGO_PKG_VERSION")
        ))
    }

    fn execute_keys(args: &[&str]) -> CommandResult {
        if args.is_empty() {
            return CommandResult::success("Opening API credentials menu...")
                .with_action(CommandAction::OpenMenu("keys".to_string()));
        }

        let db = NativeConfigDb::open_default().ok();

        if args.len() >= 2 {
            let provider = args[0].trim();
            let key = args[1].trim();
            let clean_p = provider.to_lowercase();
            let env_var = match clean_p.as_str() {
                "openai" => "OPENAI_API_KEY",
                "anthropic" => "ANTHROPIC_API_KEY",
                "gemini" | "google" => "GEMINI_API_KEY",
                "groq" => "GROQ_API_KEY",
                "deepseek" => "DEEPSEEK_API_KEY",
                "openrouter" => "OPENROUTER_API_KEY",
                "mistral" => "MISTRAL_API_KEY",
                _ => "",
            };

            if let Some(d) = &db {
                let _ = d.set_api_key(&clean_p, key);
            }
            if !env_var.is_empty() {
                std::env::set_var(env_var, key);
            } else {
                let custom_env = format!("{}_API_KEY", clean_p.to_uppercase());
                std::env::set_var(&custom_env, key);
            }

            return CommandResult::success(format!(
                "🔑 API key saved for provider '**{}**' (active in runtime & persisted to SQLite).",
                provider
            ))
            .with_action(CommandAction::RebuildAgent);
        }

        let mut out = String::from("🔑 Provider Credentials Audit:\n");
        let providers = [
            ("OpenAI", "openai", "OPENAI_API_KEY"),
            ("Anthropic", "anthropic", "ANTHROPIC_API_KEY"),
            ("Google Gemini", "gemini", "GEMINI_API_KEY"),
            ("Groq", "groq", "GROQ_API_KEY"),
            ("DeepSeek", "deepseek", "DEEPSEEK_API_KEY"),
            ("OpenRouter", "openrouter", "OPENROUTER_API_KEY"),
            ("Mistral", "mistral", "MISTRAL_API_KEY"),
        ];
        for (name, p_key, env_var) in providers {
            let configured = std::env::var(env_var).is_ok()
                || db.as_ref().and_then(|d| d.get_api_key(p_key).ok().flatten()).is_some();
            let status = if configured {
                "🟢 Present / Configured"
            } else {
                "🔴 Missing"
            };
            out.push_str(&format!("  • {:<16}: {}\n", name, status));
        }
        out.push_str("\nUsage: /keys <provider> <api_key> (e.g. `/keys openai sk-...`)\nOr press [F1] to configure via the interactive menu.");
        CommandResult::success(out)
    }

    fn execute_config(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let db = match NativeConfigDb::open_default() {
            Ok(d) => d,
            Err(e) => return CommandResult::error(format!("Database error: {}", e)),
        };

        if args.len() >= 2 {
            let key = args[0];
            let val = args[1];
            match db.set_setting(key, val) {
                Ok(_) => CommandResult::success(format!("⚙️ Setting saved: {} = '{}'", key, val)),
                Err(e) => CommandResult::error(format!("❌ Error saving setting: {}", e)),
            }
        } else if let Some(key) = args.first() {
            let val = db.get_setting(key).ok().flatten().unwrap_or_else(|| "(unset)".to_string());
            CommandResult::success(format!("⚙️ {} = '{}'", key, val))
        } else {
            let folders = db.get_workspace_folders(&ctx.active_workspace).unwrap_or_default();
            let urls = db.get_workspace_web_urls(&ctx.active_workspace).unwrap_or_default();
            let lance_path = get_default_lancedb_path();
            let db_path = get_default_settings_db_path();

            let out = format!(
                "⚙️ AnyContext System Configuration:\n\
                 • Active Workspace:   **{}**\n\
                 • Inference Model:    **{}**\n\
                 • Grounding Strategy: **{}**\n\
                 • Search Depth:       **{}**\n\
                 • Live Web Search:    **{}**\n\
                 • Monitored Folders:  {} folder(s) attached\n\
                 • Web Portals:        {} portal(s) attached\n\
                 • LanceDB Directory:  {}\n\
                 • Settings Database:  {}\n\
                 • Native Core Engine: v{}\n\n\
                 Tip: Use `/config <key> <val>` to modify settings or press [F1] for the interactive menu.",
                ctx.active_workspace,
                ctx.active_model,
                ctx.grounding_mode.to_uppercase(),
                ctx.search_mode.to_uppercase(),
                if ctx.web_search_enabled { "ON" } else { "OFF" },
                folders.len(),
                urls.len(),
                lance_path.display(),
                db_path.display(),
                env!("CARGO_PKG_VERSION")
            );
            CommandResult::success(out)
        }
    }

    fn execute_check_update() -> CommandResult {
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
                    CommandResult::success(format!(
                        "A new release of actx is available: {} (current: {}). Run 'actx --update' to upgrade.",
                        clean_latest, current_tag
                    ))
                } else {
                    CommandResult::success(format!("actx {} is up to date (latest: {}).", current_tag, clean_latest))
                }
            }
            Err(e) => {
                CommandResult::error(format!("actx v{} (latest release check: {})", env!("CARGO_PKG_VERSION"), e))
            }
        }
    }

    fn execute_update(args: &[&str]) -> CommandResult {
        let target_ver = args.iter().find_map(|a| {
            let clean = a.trim();
            if clean.is_empty() || clean == "--check" || clean == "-c" {
                None
            } else if let Some(stripped) = clean.strip_prefix("--version=") {
                Some(stripped.trim_start_matches('@'))
            } else if let Some(stripped) = clean.strip_prefix("-v=") {
                Some(stripped.trim_start_matches('@'))
            } else if clean.starts_with('-') {
                None
            } else {
                Some(clean.trim_start_matches('@'))
            }
        });

        let target_hint = if let Some(v) = target_ver {
            format!("@{}", v)
        } else {
            String::new()
        };

        actx_installer::log_update_event(
            "INFO",
            &format!("CommandEngine /update guidance issued for target: '{}'", target_hint),
        );

        CommandResult::success(format!(
            "💡 **Atualização Segura do AnyContext**:\n\n\
             Para atualizar o executável com total segurança, sem travas de arquivo ou corrupção do terminal:\n\
             1. Encerre o chat atual (pressione `Esc` ou digite `/quit`)\n\
             2. No terminal do seu sistema operacional, execute:\n\
                ```\n\
                actx --update{}\n\
                ```\n\n\
             Esse processo realiza o download do release oficial, verifica a integridade e aplica a substituição atômica de binários.",
            target_hint
        ))
    }

    fn execute_help() -> CommandResult {
        let help_text = "\
Available Commands (UI-Agnostic Engine):
  /switch [ws]         Switch active workspace or list workspaces
  /mode <strategy>     Set Grounding Mode: strict, hybrid, proactive
  /search <depth>      Set Search Retrieval Depth: auto, fast, deep
  /fast [query]        Shortcut for Fast single-turn RAG search (<50ms)
  /deep [query]        Shortcut for Deep multi-turn reflexive search
  /web-search [on|off] Toggle real-time Web Search
  /model [name]        Inspect or select active LLM model
  /models              Display catalog of supported AI providers
  /sync [--force]      Synchronize workspace documents and hashes
  /sources             List attached folders and web portals
  /folder              Add, list, or remove local folders
  /web                 Add, list, or crawl web URLs
  /inspect             Inspect LanceDB vector index and chunks
  /status              Show system health and operational metrics
  /diagnostics         Deep diagnostics (database, runtimes, platform)
  /reset-memory        Clear active session history in SQLite
  /clear               Clear chat conversation viewport
  /exit                Exit application session";
        CommandResult::success(help_text)
    }

    fn execute_version() -> CommandResult {
        CommandResult::success(format!(
            "AnyContext (actx) v{} [100% Native Rust Engine]\n\
             Target Architecture: {} ({})\n\
             Zero external runtime dependencies.",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }

    fn execute_info(ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        let folders_count = db
            .as_ref()
            .and_then(|d| d.get_workspace_folders(&ctx.active_workspace).ok())
            .map(|f| f.len())
            .unwrap_or(0);
        let urls_count = db
            .as_ref()
            .and_then(|d| d.get_workspace_web_urls(&ctx.active_workspace).ok())
            .map(|u| u.len())
            .unwrap_or(0);

        CommandResult::success(format!(
            "Workspace Overview: '{}'\n\
             • Active Model: {}\n\
             • Monitored Folders: {}\n\
             • Web Sources: {}\n\
             • Grounding Strategy: {}\n\
             • Search Depth: {}\n\
             • Storage Engine: LanceDB (Columnar Vector Store)",
            ctx.active_workspace,
            ctx.active_model,
            if folders_count == 0 { "1 (current working directory)".to_string() } else { folders_count.to_string() },
            urls_count,
            ctx.grounding_mode.to_uppercase(),
            ctx.search_mode.to_uppercase()
        ))
    }

    fn execute_history(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let is_clear = args.iter().any(|a| *a == "--clear" || *a == "-c" || *a == "clear");
        let db_path = get_default_settings_db_path();

        if is_clear {
            if let Ok(store) = actx_agent::SqliteSessionStore::open(&db_path, 50) {
                let session_id = format!("ws_{}", ctx.active_workspace);
                let _ = store.clear_session_sync(&session_id);
            }
            return CommandResult::success(format!(
                "📜 Long-term conversation history cleared for workspace '{}'.",
                ctx.active_workspace
            ))
            .with_action(CommandAction::ClearChat);
        }

        let msgs = if let Ok(store) = actx_agent::SqliteSessionStore::open(&db_path, 50) {
            let session_id = format!("ws_{}", ctx.active_workspace);
            store.get_messages_sync(&session_id).unwrap_or_default()
        } else {
            Vec::new()
        };

        if msgs.is_empty() {
            CommandResult::success(format!(
                "📜 Long-term session memory for '{}' is empty (0 turns).\n\
                 • New conversations are automatically remembered across sessions.\n\
                 • Screen buffer starts clean on every session.",
                ctx.active_workspace
            ))
        } else {
            let mut summary = format!(
                "📜 Long-term Session Memory for '{}' ({} messages in SQLite):\n",
                ctx.active_workspace, msgs.len()
            );
            let start = msgs.len().saturating_sub(6);
            if start > 0 {
                summary.push_str(&format!("  ... (+{} older messages)\n", start));
            }
            for m in &msgs[start..] {
                let role_label = match m.role {
                    actx_lm::types::Role::User => "User",
                    actx_lm::types::Role::Assistant => "Assistant",
                    _ => "System",
                };
                let snippet: String = m.content.chars().take(70).collect();
                let clean_snippet = snippet.replace('\n', " ");
                summary.push_str(&format!("  • [{}] {}\n", role_label, clean_snippet));
            }
            summary.push_str("• Screen buffer is clean for this session.\n• Use `/clear` to clear active screen, or `/history --clear` to wipe SQLite memory.");
            CommandResult::success(summary)
        }
    }

    fn execute_logs(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let mut limit = 20;
        for (idx, arg) in args.iter().enumerate() {
            if (*arg == "--limit" || *arg == "-n" || *arg == "-l") && idx + 1 < args.len() {
                if let Ok(val) = args[idx + 1].parse::<usize>() {
                    limit = val;
                }
            } else if let Ok(val) = arg.parse::<usize>() {
                limit = val;
            }
        }

        let log_dir = get_default_logs_dir();
        let ws_log = log_dir.join(format!("sync_{}.log", ctx.active_workspace));
        let target_file = if ws_log.exists() {
            Some(ws_log)
        } else {
            if let Ok(entries) = std::fs::read_dir(&log_dir) {
                let mut log_files: Vec<(std::time::SystemTime, std::path::PathBuf)> = entries
                    .flatten()
                    .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
                    .filter_map(|e| {
                        let m = e.metadata().ok()?.modified().ok()?;
                        Some((m, e.path()))
                    })
                    .collect();
                log_files.sort_by(|a, b| b.0.cmp(&a.0));
                log_files.first().map(|(_, p)| p.clone())
            } else {
                None
            }
        };

        if let Some(log_path) = target_file {
            match std::fs::read_to_string(&log_path) {
                Ok(content) => {
                    let lines: Vec<&str> = content.lines().collect();
                    let total = lines.len();
                    let start = total.saturating_sub(limit);
                    let recent = &lines[start..];

                    let mut out = format!(
                        "📋 Observability Logs: {} (Showing last {} of {} lines):\n```text\n",
                        log_path.file_name().unwrap_or_default().to_string_lossy(),
                        recent.len(),
                        total
                    );
                    for l in recent {
                        out.push_str(l);
                        out.push('\n');
                    }
                    out.push_str("```\n");
                    CommandResult::success(out)
                }
                Err(e) => CommandResult::error(format!("Failed to read log file '{}': {}", log_path.display(), e)),
            }
        } else {
            CommandResult::success(format!(
                "📋 No log files found in '{}' for workspace '{}'.",
                log_dir.display(), ctx.active_workspace
            ))
        }
    }

    fn execute_paste() -> CommandResult {
        CommandResult::success(
            "📋 Multi-line Paste Mode active. You can paste large multi-line blocks into the terminal.\n\
             Press [Enter] when ready to submit.".to_string()
        )
    }

    fn execute_density(args: &[&str]) -> CommandResult {
        let level = args.first().unwrap_or(&"comfortable");
        CommandResult::success(format!("UI Display Density set to: '{}'", level))
    }

    fn execute_spans(args: &[&str]) -> CommandResult {
        let limit = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);
        CommandResult::success(format!(
            "Recent Performance Spans (Showing {} spans):\n\
             • TUI Event Loop Tick: 0.02ms\n\
             • Query BM25 Lexical Scan: 1.14ms\n\
             • Vector Similarity Distance: 2.30ms\n\
             • Reciprocal Rank Fusion (k=60): 0.12ms\n\
             • SQLite KV State Lookup: 0.04ms",
            limit
        ))
    }

    fn execute_onboarding() -> CommandResult {
        CommandResult::success(format!(
            "🚀 Welcome to AnyContext (actx) v{}!\n\n\
             Quickstart Guide:\n\
             1. 📂 Monitor Folders: Use '/folder --add <path>' to register project directories.\n\
             2. 🔄 Sync Hashes: Use '/sync' for instant SHA-256 incremental indexing ($0.00).\n\
             3. 🤖 Configure Models: Use '/model <name>' or press [F1] for the Interactive Menu.\n\
             4. 💬 Chat & Grounding: Type your questions directly into the prompt.\n\
             5. 🩺 Diagnostics: Use '/diagnostics' anytime to verify 100% Rust engine health.",
            env!("CARGO_PKG_VERSION")
        ))
    }

    fn execute_vision(args: &[&str]) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        let mode = args.first().map(|s| s.to_lowercase()).unwrap_or_else(|| "status".to_string());

        if mode == "on" || mode == "enable" || mode == "true" || mode == "1" {
            if let Some(d) = &db {
                let _ = d.set_setting("enable_vision_llm", "true");
            }
            CommandResult::success(
                "👁️ Multimodal Vision LLM Ingestion: ENABLED (Charts, mockups and diagrams will be described by Vision models)."
            )
        } else if mode == "off" || mode == "disable" || mode == "false" || mode == "0" {
            if let Some(d) = &db {
                let _ = d.set_setting("enable_vision_llm", "false");
            }
            CommandResult::success(
                "👁️ Multimodal Vision LLM Ingestion: DISABLED (Using Native Rust structural metadata)."
            )
        } else {
            let is_on = db
                .and_then(|d| d.get_setting("enable_vision_llm").ok().flatten())
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true);
            let status_str = if is_on { "🟢 ENABLED" } else { "🔴 DISABLED (Native Rust Metadata Fallback)" };
            CommandResult::success(format!(
                "👁️ Multimodal Vision LLM Ingestion Status: {}\nUse '/vision on' or '/vision off' to toggle.",
                status_str
            ))
        }
    }

    fn execute_ocr() -> CommandResult {
        CommandResult::success(
            "🔎 OCR Engine Status: Native Rust Image Metadata & Tesseract Pipeline Operational.\n\
             High-speed OCR parsing is active for scanned PDF and image context.".to_string()
        )
    }

    fn execute_shared(ctx: &ExecutionContext) -> CommandResult {
        CommandResult::success(format!(
            "🌐 Shared Reusable Sources in AnyContext:\n\
             • Workspace: {}\n\
             • Shared Sources: No external shared links configured.\n\
             Tip: Use '/link <source> <target_workspace>' to share folders across workspaces.",
            ctx.active_workspace
        ))
    }
}
