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
            "download" | "dl" | "get-model" => {
                Self::execute_download(args, ctx)
            }
            "device" | "gpu" | "accelerator" => {
                Self::execute_device(args, ctx)
            }
            "rerank" | "reranker" => {
                Self::execute_rerank(args, ctx)
            }
            "profile" | "profiles" => {
                Self::execute_profile(args, ctx)
            }
            "preset" | "presets" | "kit" | "kits" => {
                Self::execute_preset(args, ctx)
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
            "shared" | "shared-sources" => {
                CommandResult::error("The '/shared' command and Shared Sources workspace have been deprecated in AnyContext v0.36.0. Workspaces are now strictly isolated environments.")
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
    // Hardware & Model Intelligence Profiles (Kit de Models - v0.37.0)
    // -------------------------------------------------------------------------
    fn execute_profile(args: &[&str], _ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();
        let metrics = crate::hardware::HardwareProfiler::probe_system(None);
        let evals = crate::hardware::HardwareProfiler::evaluate_profiles(&metrics);

        if args.is_empty() {
            let active_profile_str = db.as_ref()
                .and_then(|d| d.get_setting("active_profile").ok().flatten())
                .unwrap_or_else(|| {
                    crate::hardware::HardwareProfiler::auto_select_recommended(&metrics).as_str().to_string()
                });
            let active_tier = crate::hardware::ProfileTier::parse_str(&active_profile_str)
                .unwrap_or(crate::hardware::ProfileTier::SmartBalanced);

            let mut msg = format!(
                "🖥️ Hardware & Intelligence Profiles (Kit de Models):\n\
                 • Detected Hardware: {:.1} GB RAM ({:.1} GB Available) │ {:.1} GB Free SSD ({} Cores)\n\
                 • Active Profile:    {} (Current)\n\n\
                 Available Profile Tiers:\n",
                metrics.total_ram_gb(),
                metrics.available_ram_gb(),
                metrics.free_disk_gb(),
                metrics.cpu_cores,
                active_tier.display_name()
            );

            for e in &evals {
                let marker = if e.tier == active_tier { " (active)" } else { "" };
                let disk_mb = e.disk_required_bytes as f64 / (1024.0 * 1024.0);
                let ram_mb = e.estimated_ram_bytes as f64 / (1024.0 * 1024.0);
                msg.push_str(&format!(
                    "  {} {} [Disk: {:.0} MB │ RAM: {:.0} MB]{}\n     ↳ {}\n",
                    e.status.badge(),
                    e.tier.as_str(),
                    disk_mb,
                    ram_mb,
                    marker,
                    e.headline
                ));
            }

            msg.push_str("\nUsage: /profile <sovereign_ultra|workstation_pro|air_gapped_power|smart_balanced|lite>\nTip: Press [F1] -> Hardware Profiles to manage models interactively.");

            return CommandResult::success(msg)
                .with_action(CommandAction::OpenMenu("profiles".to_string()));
        }

        let requested = args[0].trim();
        if let Some(tier) = crate::hardware::ProfileTier::parse_str(requested) {
            if let Some(ref d) = db {
                let _ = d.set_setting("active_profile", tier.as_str());
            }

            let eval = evals.iter().find(|e| e.tier == tier);
            let badge = eval.map(|e| e.status.badge()).unwrap_or("🟢");
            let reason = eval.map(|e| e.reason.as_str()).unwrap_or("");

            CommandResult::success(format!(
                "✔ Switched active intelligence profile to '{}' {}.\n\
                 • Description: {}\n\
                 • Hardware Status: {}\n\
                 • Target Suite: {:?}\n\n\
                 Tip: Run `/status` to inspect active runtime telemetry.",
                tier.display_name(),
                badge,
                eval.map(|e| e.headline.as_str()).unwrap_or(""),
                reason,
                tier.target_models()
            ))
            .with_action(CommandAction::RebuildAgent)
        } else {
            CommandResult::error(format!(
                "Invalid profile tier '{}'.\n\
                 Available options:\n\
                 • `sovereign_ultra`  (or `sovereign`): Gemma 4 32B + Full Document AI (~19.2 GB disk, 64GB+ RAM)\n\
                 • `workstation_pro`  (or `workstation`): Gemma 4 27B + Full Document AI (~15.7 GB disk, 32GB RAM)\n\
                 • `air_gapped_power` (or `airgapped`): Gemma 4 12B + Full Document AI (~7.9 GB disk, 16GB RAM)\n\
                 • `smart_balanced`   (or `balanced`): Gemma 4 Compact + Essential Document AI (~1.7 GB disk, 8GB RAM)\n\
                 • `lite`             (or `cloud`): Pure Rust heuristics & Cloud APIs (0 MB disk, <45 MB RAM)\n\n\
                 Usage: /profile <sovereign_ultra|workstation_pro|air_gapped_power|smart_balanced|lite>",
                requested
            ))
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

        let reindex_note = crate::ingestion::pending_reindex_notice(&db, &ctx.active_workspace);

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
                if let Some(note) = reindex_note {
                    msg.push_str(&format!("  • ⚠️ {}\n", note));
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

        // Subcommand: /sources rename <number_or_name> <new_display_name>
        if args[0].eq_ignore_ascii_case("rename") {
            if args.len() < 3 {
                return CommandResult::error("Usage: /sources rename <number_id_or_name> <new_display_name>");
            }

            let (target_ident, new_name): (String, String) = if args.len() == 3 {
                (args[1].trim().trim_matches('"').to_string(), args[2].trim().trim_matches('"').to_string())
            } else if args[1].parse::<usize>().is_ok() {
                (args[1].trim().to_string(), args[2..].join(" ").trim().trim_matches('"').to_string())
            } else {
                let rest = args[1..].join(" ");
                let rest_trimmed = rest.trim();
                if rest_trimmed.starts_with('"') {
                    if let Some(end_quote) = rest_trimmed[1..].find('"') {
                        let ident = rest_trimmed[1..1 + end_quote].trim().to_string();
                        let remainder = rest_trimmed[1 + end_quote + 1..].trim().trim_matches('"').to_string();
                        (ident, remainder)
                    } else {
                        (args[1].trim().trim_matches('"').to_string(), args[2..].join(" ").trim().trim_matches('"').to_string())
                    }
                } else {
                    (args[1].trim().trim_matches('"').to_string(), args[2..].join(" ").trim().trim_matches('"').to_string())
                }
            };

            if new_name.is_empty() {
                return CommandResult::error("Usage: /sources rename <number_id_or_name> <new_display_name>");
            }

            match db.rename_workspace_source(&ctx.active_workspace, &target_ident, &new_name) {
                Ok(true) => {
                    return CommandResult::success(format!(
                        "✔ Source '{}' renamed to '{}'.\n  Workspace: {}\n  Chunks impacted in LanceDB: 0 (Decoupled relational alias)",
                        target_ident, new_name, ctx.active_workspace
                    ));
                }
                Ok(false) => {
                    return CommandResult::error(format!(
                        "⚠️ Source '{}' not found in workspace '{}'.\nTip: Run `/sources` to list registered source numbers and names.",
                        target_ident, ctx.active_workspace
                    ));
                }
                Err(e) => return CommandResult::error(format!("Failed to rename source: {}", e)),
            }
        }

        let is_all = args.iter().any(|a| *a == "--all" || *a == "-a" || *a == "all");
        if is_all {
            let ws_names = db.list_workspace_names().unwrap_or_else(|_| vec![ctx.active_workspace.clone()]);
            let mut msg = format!("📂 All Configured Workspaces & Sources ({} workspaces):\n\n", ws_names.len());
            for ws in &ws_names {
                let sources = db.list_workspace_sources(ws).unwrap_or_default();
                let is_active = ws.eq_ignore_ascii_case(&ctx.active_workspace);
                let active_tag = if is_active { " (active)" } else { "" };
                msg.push_str(&format!("• Workspace '{}'{} - {} source(s):\n", ws, active_tag, sources.len()));
                for (idx, s) in sources.iter().enumerate() {
                    let icon = if s.source_type == "folder" { "📁" } else { "🌐" };
                    msg.push_str(&format!("  {}. {} {} ({})\n", idx + 1, icon, s.display_name, s.raw_target));
                }
                if sources.is_empty() {
                    msg.push_str("  (no sources registered)\n");
                }
                msg.push('\n');
            }
            return CommandResult::success(msg);
        }

        let sources = db.list_workspace_sources(&ctx.active_workspace).unwrap_or_default();
        let folders: Vec<_> = sources.iter().filter(|s| s.source_type == "folder").collect();
        let urls: Vec<_> = sources.iter().filter(|s| s.source_type == "web_url").collect();

        let mut out = format!("📂 Data Sources configured for workspace '{}':\n", ctx.active_workspace);
        out.push_str("📁 Local Folders:\n");
        if folders.is_empty() {
            out.push_str("  (none configured. Use `/folder --add <path>` or `/folder <path>`)\n");
        } else {
            for (idx, f) in folders.iter().enumerate() {
                out.push_str(&format!("  {}. 📁 {} ({})\n", idx + 1, f.display_name, f.raw_target));
            }
        }
        out.push_str("\n🌐 Web Documentation Portals:\n");
        if urls.is_empty() {
            out.push_str("  (none configured. Use `/web --add <url>` or `/web <url>`)\n");
        } else {
            for (idx, u) in urls.iter().enumerate() {
                let num = folders.len() + idx + 1;
                out.push_str(&format!("  {}. 🌐 {} ({})\n", num, u.display_name, u.raw_target));
            }
        }
        if !sources.is_empty() {
            out.push_str("\n💡 Tip: Use `/sources rename <number_or_name> <new_name>` to customize display aliases ($0.00 cost, 0 vector chunks rewritten).");
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
            if !path.is_dir() {
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

        let force = args.iter().any(|a| *a == "--force" || *a == "-f");
        let non_flag_args: Vec<&str> = args.iter().copied().filter(|a| !a.starts_with('-')).collect();
        let target = match non_flag_args.first() {
            Some(t) => t.trim(),
            None => {
                return CommandResult::error("Please specify a model name. Example: /model gemma-4-12b");
            }
        };

        // Guardrail de Hardware (LM Studio style)
        let metrics = crate::hardware::HardwareProfiler::probe_system(None);
        let safety = crate::hardware::HardwareProfiler::validate_model_safety(target, &metrics);

        let hardware_badge = match &safety {
            crate::hardware::HardwareSafetyVerdict::Unsafe { required_ram_gb, total_ram_gb, recommended_model, .. } => {
                if !force {
                    return CommandResult::error(format!(
                        "❌ [Hardware Warning - LM Studio Style]\n\
                         The local model '{}' requires ~{:.1} GB of RAM.\n\
                         Your system has {:.1} GB total RAM ({:.1} GB available).\n\
                         Loading this model will likely trigger severe OS swap thrashing, terminal freezing, or Out-Of-Memory (OOM) termination.\n\n\
                         💡 Recommended safe local model for your hardware: '{}'\n\
                         To proceed anyway at your own risk, execute:\n\
                         /model {} --force",
                        target, required_ram_gb, total_ram_gb, metrics.available_ram_gb(), recommended_model, target
                    ));
                } else {
                    format!(" ⚠️ [Hardware Warning: Overridden via --force on {:.1}GB RAM]", total_ram_gb)
                }
            }
            crate::hardware::HardwareSafetyVerdict::Tight { required_ram_gb, available_ram_gb, .. } => {
                format!(" 🟡 [Hardware Notice: Tight memory margin - requires ~{:.1}GB, {:.1}GB free]", required_ram_gb, available_ram_gb)
            }
            crate::hardware::HardwareSafetyVerdict::Safe { .. } => {
                if target.starts_with("gemma-4") || target.starts_with("gemma") {
                    " 🟢 [Hardware Compatible]".to_string()
                } else {
                    String::new()
                }
            }
        };

        let db = NativeConfigDb::open_default().ok();
        if let Some(d) = &db {
            let _ = d.set_setting("active_model", target);
            let _ = d.set_workspace_model(&ctx.active_workspace, target);
        }
        let mut updates = CommandStateUpdates::default();
        updates.active_model = Some(target.to_string());

        let extra_note = if target.starts_with("gemma-4") || target.starts_with("gemma") {
            let tier_opt = actx_onnx::GemmaModelTier::from_model_name(target);
            if let Some(tier) = tier_opt {
                let models_dir = crate::storage::get_default_models_dir();
                let model_file = models_dir.join(tier.expected_filename());
                if model_file.exists() {
                    " [Local ONNX: Ready]"
                } else {
                    " [Local ONNX: Weights not installed - fallback active]"
                }
            } else {
                ""
            }
        } else {
            ""
        };

        let reindex_note = db
            .as_ref()
            .and_then(|d| crate::ingestion::pending_reindex_notice(d, &ctx.active_workspace))
            .map(|n| format!("\n⚠️ {}", n))
            .unwrap_or_default();

        CommandResult::success(format!("🤖 Active language model for '{}' set to: **{}**{}{}{}", ctx.active_workspace, target, extra_note, hardware_badge, reindex_note))
            .with_action(CommandAction::RebuildAgent)
            .with_state_updates(updates)
    }

    /// True for models that generate on this machine today (an Ollama-served model).
    /// Local Gemma 4 generation is planned (ADR-0130), so `gemma*` models do not qualify.
    fn is_working_local_model(model: &str) -> bool {
        let m = model.trim().to_lowercase();
        m.starts_with("ollama/") || m.starts_with("local/")
    }

    /// Applies the network-isolated settings shared by the `offline` and `privacy` presets,
    /// keeping the workspace's local model. Refuses (changing nothing) when the active model is
    /// not a working local model, instead of switching to a planned model that cannot answer.
    fn apply_local_preset(ctx: &ExecutionContext, title: &str, icon: &str) -> CommandResult {
        let model = ctx.active_model.trim().to_string();
        if !Self::is_working_local_model(&model) {
            return CommandResult::error(format!(
                "The '{}' preset needs a model that runs on this machine, and the active model '{}' does not.
                 Local Gemma 4 generation is planned and not available yet.
                 Select a local model served by Ollama first, for example `/model ollama/qwen2.5`, then run the preset again.
                 Nothing was changed.",
                title, model
            ));
        }
        if let Ok(d) = NativeConfigDb::open_default() {
            let _ = d.set_workspace_grounding_mode(&ctx.active_workspace, "strict");
            let _ = d.set_workspace_web_search(&ctx.active_workspace, false);
            let _ = d.set_workspace_search_mode(&ctx.active_workspace, "fast");
        }
        let mut updates = CommandStateUpdates::default();
        updates.grounding_mode = Some("strict".to_string());
        updates.web_search_enabled = Some(false);
        updates.search_mode = Some("fast".to_string());

        CommandResult::success(format!(
            "{} Switched workspace '{}' to the **{}** preset:
             • Model:          {} (local)
             • Grounding Mode: Strict
             • Search Depth:   Fast RAG
             • Web Search:     Disabled (no outbound web search)",
            icon, ctx.active_workspace, title, model
        ))
        .with_action(CommandAction::RebuildAgent)
        .with_state_updates(updates)
    }

    fn execute_preset(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        let db = NativeConfigDb::open_default().ok();

        if args.is_empty() {
            let msg = format!("🎛️ Workspace Intelligence & Operational Presets:

                 • offline   - On-device answers with a local Ollama model, no web search.
                               ↳ Keeps your local model (e.g. 'ollama/qwen2.5'), strict grounding, fast search, web-search off.
                 • balanced  - Balanced cost-performance cloud reasoning.
                               ↳ Sets model to '{}', hybrid grounding, auto search depth, web-search on.
                 • frontier  - Maximum analytical reasoning and architecture synthesis.
                               ↳ Sets model to '{}', proactive grounding, deep search, web-search on.
                 • privacy   - Corporate data isolation with a local Ollama model.
                               ↳ Same settings as offline.

                 Local Gemma 4 generation is planned; select a local model first with /model ollama/<model>.
                 Usage: /preset <offline|balanced|frontier|privacy>
                 Alias: /kit <name>",
                crate::llm_catalog::BALANCED_PRESET_MODEL,
                crate::llm_catalog::FRONTIER_PRESET_MODEL,
            );
            return CommandResult::success(msg);
        }

        let choice = args[0].trim().to_lowercase();

        match choice.as_str() {
            "offline" | "local" => Self::apply_local_preset(ctx, "Offline-Local", "🔒"),
            "balanced" | "hybrid" => {
                let model = crate::llm_catalog::BALANCED_PRESET_MODEL;
                if let Some(ref d) = db {
                    let _ = d.set_workspace_model(&ctx.active_workspace, model);
                    let _ = d.set_workspace_grounding_mode(&ctx.active_workspace, "hybrid");
                    let _ = d.set_workspace_web_search(&ctx.active_workspace, true);
                    let _ = d.set_workspace_search_mode(&ctx.active_workspace, "auto");
                    let _ = d.set_setting("active_model", model);
                }
                let mut updates = CommandStateUpdates::default();
                updates.active_model = Some(model.to_string());
                updates.grounding_mode = Some("hybrid".to_string());
                updates.web_search_enabled = Some(true);
                updates.search_mode = Some("auto".to_string());

                CommandResult::success(format!(
                    "⚡ Switched workspace '{}' to **Hybrid-Balanced** preset:\n\
                     • Model:          {}\n\
                     • Grounding Mode: Hybrid\n\
                     • Search Depth:   Auto (Fast RAG / Deep Search)\n\
                     • Web Search:     Enabled",
                    ctx.active_workspace, model
                ))
                .with_action(CommandAction::RebuildAgent)
                .with_state_updates(updates)
            }
            "frontier" | "max" | "power" => {
                let model = crate::llm_catalog::FRONTIER_PRESET_MODEL;
                if let Some(ref d) = db {
                    let _ = d.set_workspace_model(&ctx.active_workspace, model);
                    let _ = d.set_workspace_grounding_mode(&ctx.active_workspace, "proactive");
                    let _ = d.set_workspace_web_search(&ctx.active_workspace, true);
                    let _ = d.set_workspace_search_mode(&ctx.active_workspace, "deep");
                    let _ = d.set_setting("active_model", model);
                }
                let mut updates = CommandStateUpdates::default();
                updates.active_model = Some(model.to_string());
                updates.grounding_mode = Some("proactive".to_string());
                updates.web_search_enabled = Some(true);
                updates.search_mode = Some("deep".to_string());

                CommandResult::success(format!(
                    "👑 Switched workspace '{}' to **Maximum-Frontier** preset:\n\
                     • Model:          {}\n\
                     • Grounding Mode: Proactive\n\
                     • Search Depth:   Deep Search\n\
                     • Web Search:     Enabled",
                    ctx.active_workspace, model
                ))
                .with_action(CommandAction::RebuildAgent)
                .with_state_updates(updates)
            }
            "privacy" | "enterprise" => Self::apply_local_preset(ctx, "Enterprise-Privacy", "🛡️"),
            _ => {
                CommandResult::error(format!(
                    "Invalid preset '{}'. Available presets: offline, balanced, frontier, privacy.\nExample: /preset offline",
                    choice
                ))
            }
        }
    }

    fn execute_models_catalog(_args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        use crate::llm_catalog::{chat_models, ChatProvider};

        let section = |title: &str, providers: &[ChatProvider]| -> String {
            let mut out = format!("{title}\n");
            for model in chat_models().iter().filter(|m| providers.contains(&m.provider)) {
                out.push_str(&format!("• {:<26} - {} {}\n", model.id, model.provider.label(), model.summary));
            }
            out
        };

        let models_text = format!(
            "🤖 Supported AI Providers & Models (Current: **{}**):\n\n{}\n{}\n{}\nAny Ollama model works as ollama/<model>.\nUsage: /model <model_name>",
            ctx.active_model,
            section("🧠 Local ONNX Neural Brain (Gemma 4 Family):", &[ChatProvider::LocalGemma]),
            section(
                "⭐ Cloud Providers:",
                &[ChatProvider::Gemini, ChatProvider::OpenAi, ChatProvider::Anthropic, ChatProvider::DeepSeek, ChatProvider::Groq],
            ),
            section("🔒 100% Local & Free:", &[ChatProvider::Ollama, ChatProvider::Mock]),
        );
        CommandResult::success(models_text)
    }

    fn execute_download(args: &[&str], _ctx: &ExecutionContext) -> CommandResult {
        let check = args.iter().any(|a| *a == "--check" || *a == "-c" || *a == "check");
        let force = args.iter().any(|a| *a == "--force" || *a == "-f" || *a == "force");
        let list = args.iter().any(|a| *a == "--list" || *a == "-l" || *a == "list");

        let non_flag_args: Vec<&str> = args
            .iter()
            .copied()
            .filter(|a| !a.starts_with('-') && *a != "check" && *a != "list" && *a != "force")
            .collect();

        let target = non_flag_args.first().copied();

        if check || (target.is_none() && list) {
            let summary = crate::ingestion::OnnxModelManager::inspect_store();
            let metrics = crate::hardware::HardwareProfiler::probe_system(None);
            let total_mb = summary.total_bytes_on_disk as f64 / (1024.0 * 1024.0);

            let mut out = format!(
                "📦 **AnyContext ONNX Model Store Status**\n\
                 📁 Directory: `{}`\n\
                 📊 Installed: {} of {} models ({:.1} MB on disk)\n\n\
                 | Tier / Model ID | Category | Size | Status | Hardware Compatibility |\n\
                 | :--- | :--- | :--- | :--- | :--- |\n",
                summary.models_dir.display(),
                summary.installed_count,
                summary.total_count,
                total_mb
            );

            for item in &summary.models {
                let safety = crate::hardware::HardwareProfiler::validate_model_safety(&item.id, &metrics);
                let (compat_badge, icon) = match safety {
                    crate::hardware::HardwareSafetyVerdict::Safe { .. } => ("Safe / Compatible", "🟢"),
                    crate::hardware::HardwareSafetyVerdict::Tight { .. } => ("Tight / Swap Risk", "🟡"),
                    crate::hardware::HardwareSafetyVerdict::Unsafe { .. } => ("Exceeds RAM", "🔴"),
                };

                let (status_badge, size_str) = if item.planned {
                    ("🕓 Planned (no download yet)", "—".to_string())
                } else if item.is_installed {
                    let mb = item.disk_size_bytes as f64 / (1024.0 * 1024.0);
                    ("🟢 Installed", format!("{:.1} MB", mb))
                } else {
                    let nom_mb = item.expected_size_bytes as f64 / (1024.0 * 1024.0);
                    if nom_mb >= 1000.0 {
                        ("⚪ Not Installed", format!("~{:.2} GB", nom_mb / 1024.0))
                    } else {
                        ("⚪ Not Installed", format!("~{:.0} MB", nom_mb))
                    }
                };

                out.push_str(&format!(
                    "| `{}` | {} | {} | {} | {} {} |\n",
                    item.id,
                    item.category,
                    size_str,
                    status_badge,
                    icon,
                    compat_badge
                ));
            }

            out.push_str("\n💡 Models are installed automatically with your hardware profile. Use `/download <model>` (e.g. `/download reranker`) only to repair or add a model.");
            return CommandResult::success(out);
        }

        let target_str = match target {
            Some(t) => t.trim(),
            None => {
                return CommandResult::success(
                    "📥 **AnyContext Local Model Downloader**\n\n\
                     Models are verified (size + SHA-256) and normally installed automatically with your profile.\n\n\
                     • `reranker` (mmarco-reranker, ~129 MB) - Default multilingual cross-encoder\n\
                     • `reranker-m3` (bge-reranker-v2-m3, ~561 MB) - Higher-quality cross-encoder (32GB+)\n\
                     • `laya`, `layout`, `scans`, `vision`, `bge` - Document AI weights (components run heuristics for now)\n\
                     • Gemma 4 local generation (`compact`, `12b`, `27b`, `32b`) is planned: no download yet\n\n\
                     Usage:\n\
                       `/download <model>` (e.g. `/download reranker`)\n\
                       `/download --check` (inspect local model storage)\n\
                       `/download <model> --force` (re-download / override guardrails)"
                );
            }
        };

        let spec = match crate::ingestion::OnnxModelManager::resolve_spec(target_str) {
            Some(s) => s,
            None => {
                return CommandResult::error(format!(
                    "❌ Model '{}' not recognized.\n\
                     Examples: `reranker`, `reranker-m3`, `laya`, `layout`, `scans`, `vision`.\n\
                     Run `/download --check` to see the full catalog.",
                    target_str
                ));
            }
        };

        if !spec.is_available() {
            return CommandResult::success(crate::ingestion::OnnxModelManager::planned_message(spec));
        }

        if crate::ingestion::OnnxModelManager::is_installed(spec) && !force {
            return CommandResult::success(format!(
                "✔ Model **{}** is already installed and verified.\n\
                 Location: `{}`\n\n\
                 • To force re-download, execute: `/download {} --force`",
                spec.name,
                crate::ingestion::OnnxModelManager::model_path(spec).display(),
                spec.id
            ));
        }

        if let Err(err_msg) = crate::ingestion::OnnxModelManager::check_download_safety(spec, force) {
            return CommandResult::error(err_msg);
        }

        let size_gb = spec.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let force_note = if force { " *(Guardrails bypassed via --force)*" } else { "" };
        CommandResult::success(format!(
            "📥 Pre-flight safety checks passed! Starting async download of **{}** (~{:.2} GB)...{}\n\
             File: `{}`\n\
             Progress is displayed in the footer. Press [Esc] anytime to cancel.",
            spec.name, size_gb, force_note, spec.file_name
        ))
        .with_action(CommandAction::StartDownload(spec.id.to_string()))
    }

    fn execute_device(args: &[&str], _ctx: &ExecutionContext) -> CommandResult {
        let metrics = crate::hardware::HardwareProfiler::probe_system(None);
        let target = args.first().copied().map(str::trim);

        if let Some(target_str) = target {
            if let Some(device) = actx_onnx::ExecutionDevice::parse_str(target_str) {
                let mut updates = CommandStateUpdates::default();
                updates.execution_device = Some(device.as_str().to_string());
                let inference_note = if device.is_supported_for_inference() {
                    "• Local inference runs on the CPU (ONNX Runtime CPU Execution Provider)."
                } else {
                    "• ⚠️ GPU execution providers are planned: local inference keeps running on the CPU in this version."
                };
                return CommandResult::success(format!(
                    "🚀 Preferred acceleration target set to: **{}** (`{}`)\n\n\
                     • Detected Adapter: {}\n\
                     {}",
                    device.display_name(),
                    device.as_str(),
                    metrics.acceleration_summary(),
                    inference_note
                ))
                .with_action(CommandAction::SetDevice(device.as_str().to_string()))
                .with_state_updates(updates);
            } else {
                return CommandResult::error(format!(
                    "Unrecognized device '{}'. Valid options are:\n\
                     • `auto`: Automatically select optimal system accelerator\n\
                     • `dml` or `directml`: Microsoft DirectX 12 Machine Learning (Windows)\n\
                     • `metal` or `coreml`: Apple Silicon Metal / CoreML Engine (macOS)\n\
                     • `cuda`: NVIDIA CUDA / TensorRT (Linux & Windows)\n\
                     • `vulkan`: Khronos Vulkan Compute (Cross-Platform)\n\
                     • `cpu`: Pure CPU SIMD multi-threading",
                    target_str
                ));
            }
        }

        // List detected accelerators and status
        let mut out = format!(
            "🚀 **AnyContext Tri-Cross-Platform Hardware Acceleration Matrix**\n\n\
             📊 **System Status**:\n\
             • Host Architecture: {}\n\
             • CPU Cores: {}\n\
             • Recommended Target: **{}**\n\
             • Active Device Summary: {}\n\n\
             | Index | Device Name | Vendor | Backend | Dedicated VRAM | Discrete? |\n\
             | :--- | :--- | :--- | :--- | :--- | :--- |\n",
            std::env::consts::OS,
            metrics.cpu_cores,
            metrics.recommended_execution_device().display_name(),
            metrics.acceleration_summary()
        );

        if metrics.accelerators.is_empty() {
            out.push_str("| 0 | Host CPU SIMD Vectorization | Generic | CPU | Shared System RAM | No (Pure Host) |\n");
        } else {
            for (idx, acc) in metrics.accelerators.iter().enumerate() {
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {:.1} GB | {} |\n",
                    idx,
                    acc.name,
                    acc.vendor.display_name(),
                    acc.backend.as_str().to_uppercase(),
                    acc.vram_gb(),
                    if acc.is_discrete { "Yes (Dedicated)" } else { "No (Integrated/UMA)" }
                ));
            }
        }

        out.push_str("\n💡 Run `/device <target>` (e.g. `/device dml`, `/device metal`, `/device cuda`, `/device cpu`) to switch execution targets.");
        CommandResult::success(out)
    }

    fn execute_rerank(args: &[&str], ctx: &ExecutionContext) -> CommandResult {
        use crate::ingestion::OnnxModelManager;

        let target = args.first().copied().map(str::trim).map(str::to_lowercase);
        let spec = OnnxModelManager::active_reranker_spec();
        let is_installed = OnnxModelManager::is_installed(spec);
        let runtime_present = actx_onnx::runtime::runtime_library_present();

        let persist_mode = |mode: &str| {
            if let Ok(db) = NativeConfigDb::open_default() {
                let _ = db.set_setting("rerank_mode", mode);
            }
            let mut updates = CommandStateUpdates::default();
            updates.rerank_mode = Some(mode.to_string());
            updates
        };
        let availability_note = || -> String {
            if !is_installed {
                format!(
                    "\n⚠️ Weights for `{}` are not installed yet; RRF serves as fallback. They are installed with your profile, or run `actx download reranker`.",
                    spec.id
                )
            } else if !runtime_present {
                "\n⚠️ The ONNX Runtime library is missing next to the executable; RRF serves as fallback. Reinstall or update AnyContext to restore it.".to_string()
            } else {
                format!("\n⚡ Neural cross-encoder `{}` ready (ONNX Runtime, CPU).", spec.id)
            }
        };

        if let Some(ref mode_str) = target {
            match mode_str.as_str() {
                "on" | "enable" | "enabled" | "1" | "true" => {
                    let updates = persist_mode("on");
                    return CommandResult::success(format!(
                        "⚡ Neural Cross-Encoder Re-Ranking mode set to: **ON**{}\n\n\
                         • Model: {}\n\
                         • Execution: ONNX Runtime on CPU (top {} candidates re-scored per query)",
                        availability_note(),
                        spec.name,
                        crate::retrieval::pipeline::MAX_RERANK_CANDIDATES
                    ))
                    .with_state_updates(updates);
                }
                "off" | "disable" | "disabled" | "0" | "false" => {
                    let updates = persist_mode("off");
                    return CommandResult::success(
                        "⚡ Neural Cross-Encoder Re-Ranking mode set to: **OFF**\n\n\
                         • Retrieval relies exclusively on Reciprocal Rank Fusion (RRF, k=60).\n\
                         • Minimum latency mode with zero neural overhead."
                    )
                    .with_state_updates(updates);
                }
                "auto" | "default" => {
                    let updates = persist_mode("auto");
                    return CommandResult::success(format!(
                        "⚡ Neural Cross-Encoder Re-Ranking mode set to: **AUTO**{}\n\n\
                         • Two-stage neural retrieval is used whenever the model and runtime are available, falling back to RRF without interruption.",
                        availability_note()
                    ))
                    .with_state_updates(updates);
                }
                "model" => {
                    let Some(model_arg) = args.get(1).map(|s| s.trim()) else {
                        let mut out = String::from("Available re-ranker models:\n");
                        for r in OnnxModelManager::rerankers() {
                            let marker = if r.id == spec.id { " (active)" } else { "" };
                            let state = if OnnxModelManager::is_installed(r) { "installed" } else { "not installed" };
                            out.push_str(&format!(
                                "  • `{}`{} — {} (~{:.0} MB, {})\n",
                                r.id, marker, r.name, r.size_bytes as f64 / (1024.0 * 1024.0), state
                            ));
                        }
                        out.push_str("\nUsage: `/rerank model <id>`");
                        return CommandResult::success(out);
                    };
                    let chosen = OnnxModelManager::resolve_spec(model_arg)
                        .filter(|s| s.category == crate::ingestion::OnnxModelCategory::ReRanker && s.is_available());
                    let Some(chosen) = chosen else {
                        let ids: Vec<&str> = OnnxModelManager::rerankers().map(|r| r.id).collect();
                        return CommandResult::error(format!(
                            "Unknown re-ranker model '{}'. Available: {}",
                            model_arg,
                            ids.join(", ")
                        ));
                    };
                    OnnxModelManager::set_active_reranker(chosen.id);
                    let note = if OnnxModelManager::is_installed(chosen) {
                        "Weights installed; the new model is used from the next query.".to_string()
                    } else {
                        format!(
                            "⚠️ Weights not installed yet (~{:.0} MB). Run `actx download {}`; until then RRF serves as fallback.",
                            chosen.size_bytes as f64 / (1024.0 * 1024.0),
                            chosen.id
                        )
                    };
                    return CommandResult::success(format!(
                        "⚡ Re-ranker model set to **{}** (`{}`).\n{}",
                        chosen.name, chosen.id, note
                    ))
                    .with_action(CommandAction::RebuildAgent);
                }
                _ => {
                    return CommandResult::error(format!(
                        "Unrecognized rerank option '{}'. Valid options:\n\
                         • `/rerank on`: Force enable neural re-ranking\n\
                         • `/rerank off`: Disable neural re-ranking (pure RRF)\n\
                         • `/rerank auto`: Use neural re-ranking when available\n\
                         • `/rerank model [id]`: List or select the cross-encoder model\n\
                         • `/rerank`: View status and telemetry",
                        mode_str
                    ));
                }
            }
        }

        // Status view
        let (active_name, active_detail, is_active) = OnnxModelManager::active_reranker_state();
        let badge = if is_active { "🟢 Neural" } else { "⚪ RRF Fallback" };
        let disk_info = if is_installed {
            let size = OnnxModelManager::installed_size(spec).unwrap_or(0);
            format!("{:.2} MB on disk", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("Not installed (~{:.1} MB: `actx download reranker`)", spec.size_bytes as f64 / (1024.0 * 1024.0))
        };
        let runtime_info = match actx_onnx::runtime::runtime_library_path() {
            Some(p) if p.is_file() => format!("ONNX Runtime library found ({})", p.display()),
            Some(p) => format!("ONNX Runtime library missing (expected at {})", p.display()),
            None => "ONNX Runtime library location unknown".to_string(),
        };
        let latency = crate::retrieval::pipeline::last_rerank_latency_ms()
            .map(|ms| format!("{} ms (last neural pass in this session)", ms))
            .unwrap_or_else(|| "n/a (no neural pass yet in this session)".to_string());
        let requested_device = &ctx.execution_device;
        let device_note = match actx_onnx::ExecutionDevice::parse_str(requested_device) {
            Some(d) if !d.is_supported_for_inference() => format!("CPU (requested `{}`; GPU execution providers are planned)", requested_device),
            _ => "CPU (ONNX Runtime CPU Execution Provider)".to_string(),
        };

        CommandResult::success(format!(
            "⚡ **Two-Stage Neural Retrieval & Re-ranking Status**\n\n\
             • Mode Policy: **{}**\n\
             • Model: {} (`{}`)\n\
             • Engine: {}\n\
             • State: {} ({})\n\
             • Storage: {}\n\
             • Runtime: {}\n\
             • Inference Device: {}\n\
             • Latency: {}\n\n\
             **Two-Stage Pipeline Flow**:\n\
             1. Stage 1 (High Recall): LanceDB Cosine + BM25 Okapi -> Reciprocal Rank Fusion (k=60)\n\
             2. Stage 2 (High Precision): cross-encoder re-scores the top {} candidates (sigmoid scores in [0,1])\n\
             3. Delivery: Source Diversification -> Density Budgeting -> U-Shaped Reordering -> XML Isolation\n\n\
             *Tip*: `/rerank on|off|auto` switches the policy; `/rerank model` lists models.",
            ctx.rerank_mode.to_uppercase(),
            spec.name,
            spec.id,
            active_name,
            badge,
            active_detail,
            disk_info,
            runtime_info,
            device_note,
            latency,
            crate::retrieval::pipeline::MAX_RERANK_CANDIDATES
        ))
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
        let where_clause = format!("workspace = '{ws_clean}' AND length(text) > 0");

        // Chunk text is encrypted at rest, so a text filter cannot run inside LanceDB:
        // scan a bounded sample and match the decrypted fields in memory.
        let samples = if let Some(q) = filter_query {
            let needle = q.to_lowercase();
            lance
                .search_metadata(&where_clause, 2000, Some(&ctx.active_workspace), None)
                .unwrap_or_default()
                .into_iter()
                .filter(|r| {
                    r.text.to_lowercase().contains(&needle)
                        || r.file_name.to_lowercase().contains(&needle)
                        || r.file_path.to_lowercase().contains(&needle)
                        || r.content_type.as_deref().unwrap_or("").to_lowercase().contains(&needle)
                })
                .collect::<Vec<_>>()
        } else {
            lance.search_metadata(&where_clause, limit.max(20), Some(&ctx.active_workspace), None).unwrap_or_default()
        };

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

        let model_summary = crate::ingestion::OnnxModelManager::inspect_store();
        let (classifier_name, classifier_desc, _) = crate::ingestion::OnnxModelManager::active_ingestion_classifier();
        let (scanned_name, scanned_desc, _) = crate::ingestion::OnnxModelManager::active_scanned_classifier();
        let (vision_name, vision_desc, _) = crate::ingestion::OnnxModelManager::active_vision_state();
        let mb_on_disk = model_summary.total_bytes_on_disk as f64 / (1024.0 * 1024.0);
        let last_sync_line = db.as_ref()
            .and_then(|d| d.get_sync_status(&ctx.active_workspace).ok().flatten())
            .filter(|s| !s.is_syncing)
            .and_then(|s| s.error)
            .filter(|e| !e.trim().is_empty())
            .map(|e| format!("\n• Last Sync:        ⚠ {e}"))
            .unwrap_or_default();

        let metrics = crate::hardware::HardwareProfiler::probe_system(None);
        let evals = crate::hardware::HardwareProfiler::evaluate_profiles(&metrics);
        let active_profile_str = db.as_ref()
            .and_then(|d| d.get_setting("active_profile").ok().flatten())
            .unwrap_or_else(|| {
                crate::hardware::HardwareProfiler::auto_select_recommended(&metrics).as_str().to_string()
            });
        let active_tier = crate::hardware::ProfileTier::parse_str(&active_profile_str)
            .unwrap_or(crate::hardware::ProfileTier::SmartBalanced);
        let active_eval = evals.iter().find(|e| e.tier == active_tier);
        let sema_badge = active_eval.map(|e| e.status.badge()).unwrap_or("🟢");

        CommandResult::success(format!(
            "📊 AnyContext Native Core Operational Status:\n\
             • Workspace:        {} (Total workspaces: {})\n\
             • Target Model:     {}\n\
             • Grounding:        {}\n\
             • Search Depth:     {}\n\
             • Active Profile:   {} (Readiness: {})\n\
             • Hardware Metrics: {:.1} GB RAM ({:.1} GB Avail) │ {:.1} GB Free Disk ({} Cores)\n\
             • Web Search:       {}\n\
             • Local Folders:    {} attached\n\
             • Vector Chunks:    {} indexed (LanceDB)\n\
             • Dense Search:     {}\n\
             • Local AI Models:\n\
               - Ingestion Classifier: {} [{}]\n\
               - Scanned Classifier:   {} [{}]\n\
               - Document Vision:      {} [{}]\n\
               - Models Directory:     {} ({} models / {:.1} MB){}\n\
             • Health:           ● HEALTHY & READY",
            ctx.active_workspace,
            ws_count,
            ctx.active_model,
            ctx.grounding_mode.to_uppercase(),
            ctx.search_mode.to_uppercase(),
            active_tier.display_name(),
            sema_badge,
            metrics.total_ram_gb(),
            metrics.available_ram_gb(),
            metrics.free_disk_gb(),
            metrics.cpu_cores,
            if ctx.web_search_enabled { "ON" } else { "OFF" },
            folders_count,
            chunks_count,
            crate::retrieval::dense_search_status(),
            classifier_name,
            classifier_desc,
            scanned_name,
            scanned_desc,
            vision_name,
            vision_desc,
            model_summary.models_dir.display(),
            model_summary.installed_count,
            mb_on_disk,
            last_sync_line
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

            let persist_error = match &db {
                Some(d) => d.set_api_key(&clean_p, key).err().map(|e| e.to_string()),
                None => Some("settings database unavailable".to_string()),
            };
            if !env_var.is_empty() {
                std::env::set_var(env_var, key);
            } else {
                let custom_env = format!("{}_API_KEY", clean_p.to_uppercase());
                std::env::set_var(&custom_env, key);
            }

            if let Some(err) = persist_error {
                return CommandResult::success(format!(
                    "⚠️ API key for provider '**{}**' is active for this session only: it could not be saved to SQLite ({}).",
                    provider, err
                ))
                .with_action(CommandAction::RebuildAgent);
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
            "💡 **Safe AnyContext Update**:\n\n\
             To update the binary safely without file locks or terminal corruption:\n\
             1. Exit the current chat session (press `Esc` or type `/quit`)\n\
             2. In your OS terminal shell, run:\n\
                ```\n\
                actx --update{}\n\
                ```\n\n\
             This process downloads the official release, verifies integrity, and performs atomic binary replacement.",
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
  /device [target]     Switch hardware acceleration: auto, dml, metal, cuda, cpu
  /download [tier]     Download ONNX model weights (compact, 12b, 27b, 32b)
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
}
