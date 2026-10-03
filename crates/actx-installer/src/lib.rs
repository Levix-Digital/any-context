pub mod atomic_swap;
pub mod downloader;
pub mod extractor;
pub mod paths;
pub mod validator;

use std::env;
use std::path::Path;
use std::process::Command;

use atomic_swap::finalize_staging_update;
use downloader::{download_release_asset, fetch_latest_release_tag};
use extractor::extract_archive;
pub use paths::{get_canonical_bin_dir, get_core_exe_name, get_distribution_asset_name, get_shim_exe_name};
use validator::validate_binary_format;

pub const FALLBACK_VERSION: &str = "v0.30.34";

/// Executes the ultra-fast Launcher Shim or dispatches update subcommands.
pub fn run_launcher_or_update_workflow(args: &[String], current_exe: &Path) {
    let base_dir = current_exe
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(get_canonical_bin_dir);

    // 1. Ultra-fast path for version check (< 1ms)
    if args.len() > 1 && (args[1] == "-v" || args[1] == "--version") {
        let version_file = base_dir.join("version.txt");
        let version = if version_file.exists() {
            std::fs::read_to_string(&version_file)
                .map(|s| s.trim().trim_start_matches('\u{feff}').to_string())
                .unwrap_or_else(|_| FALLBACK_VERSION.to_string())
        } else {
            FALLBACK_VERSION.to_string()
        };
        let clean_version = if version.starts_with('v') || version.starts_with('V') {
            version
        } else {
            format!("v{}", version)
        };
        println!("{}", clean_version);
        std::process::exit(0);
    }

    let staging_dir = base_dir.join("actx_staging");
    let pending_flag = staging_dir.join("pending_update.json");

    // 2. Direct invocation to finalize a pending update
    if args.len() > 1 && args[1] == "--finalize-update" {
        let new_ver = read_version_from_pending_json(&pending_flag);
        match finalize_staging_update(&base_dir, &staging_dir, new_ver.as_deref()) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("[!] Error finalizing update: {}", e);
                std::process::exit(1);
            }
        }
    }

    // 3. User requested update directly from CLI
    if args.len() > 1 && (args[1] == "--update" || args[1] == "upgrade") {
        let target_ver = args.iter().find_map(|a| {
            if a.starts_with("--update@") {
                Some(a.trim_start_matches("--update@"))
            } else if a.starts_with('@') {
                Some(a.trim_start_matches('@'))
            } else {
                None
            }
        });
        run_standalone_update(&base_dir, target_ver);
        return;
    }

    // 4. Pre-flight check: finalize any orphaned pending updates and clean lingering .old files
    if pending_flag.exists() {
        let new_ver = read_version_from_pending_json(&pending_flag);
        let _ = finalize_staging_update(&base_dir, &staging_dir, new_ver.as_deref());
    }
    if let Ok(entries) = std::fs::read_dir(&base_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Some(ext) = p.extension() {
                    if ext.eq_ignore_ascii_case("old") {
                        let _ = std::fs::remove_file(&p);
                    }
                }
            }
        }
    }

    // 5. Locate and validate the heavy core engine
    let core_name = get_core_exe_name();
    let core_exe = base_dir.join(core_name);

    if !core_exe.exists() {
        eprintln!("[!] Error: AnyContext core engine ('{}') not found in: {}", core_name, base_dir.display());
        eprintln!("[>] Please run 'actx --update' or reinstall to repair.");
        std::process::exit(1);
    }

    // Binary format guard before spawning
    if let Err(e) = validate_binary_format(&core_exe) {
        eprintln!("[!] Error: AnyContext core engine is corrupted or invalid: {}", e);
        eprintln!("[>] Run 'actx --update' to download an authentic binary release.");
        std::process::exit(1);
    }

    // 6. Transparent proxy execution of core engine
    let mut cmd = Command::new(&core_exe);
    if args.len() > 1 {
        cmd.args(&args[1..]);
    }
    cmd.env("ACTX_LAUNCHER_PID", std::process::id().to_string());

    let exit_status = match cmd.status() {
        Ok(status) => status,
        Err(e) => {
            eprintln!("[!] Error executing AnyContext: {}", e);
            std::process::exit(1);
        }
    };

    let exit_code = exit_status.code().unwrap_or(0);

    // 7. Post-exit check: If update was prepared in staging or returned code 42
    if pending_flag.exists() || exit_code == 42 {
        let new_ver = read_version_from_pending_json(&pending_flag);
        let swap_res = finalize_staging_update(&base_dir, &staging_dir, new_ver.as_deref());
        match swap_res {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("[!] Error finalizing update: {}", e);
                std::process::exit(exit_code);
            }
        }
    }

    std::process::exit(exit_code);
}

/// Executes the full installer workflow for fresh installations or repairs.
pub fn run_installer_workflow(args: &[String]) {
    println!("\n=======================================================");
    println!("  AnyContext (actx) - Universal Native Installer");
    println!("  High-Performance Multi-Context Engine (Levix Digital)");
    println!("=======================================================\n");

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Usage: actx-installer [OPTIONS]\n");
        println!("Options:");
        println!("  -h, --help               Print this help menu");
        println!("  --install                Perform fresh installation / repair (default)");
        println!("  --update                 Check for and apply latest updates via HTTPS");
        println!("  --rollback               Rollback to previous installation if available");
        println!("  --version=<tag>          Install a specific version tag (e.g., --version=v0.30.35)");
        println!("  --dir=<path>             Custom installation directory\n");
        return;
    }

    let target_dir = args
        .iter()
        .find(|a| a.starts_with("--dir="))
        .and_then(|a| a.split('=').nth(1))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(get_canonical_bin_dir);

    println!("[*] Installation target directory: {}", target_dir.display());

    if args.iter().any(|a| a == "--update") {
        run_standalone_update(&target_dir, None);
        return;
    }

    if args.iter().any(|a| a == "--rollback") {
        let old_core = target_dir.join(if cfg!(windows) { "actx_old.exe" } else { "actx_old" });
        let target_core = target_dir.join(get_core_exe_name());
        if old_core.exists() {
            println!("[*] Rolling back to previous binary '{}'...", old_core.display());
            let _ = std::fs::remove_file(&target_core);
            if let Err(e) = std::fs::rename(&old_core, &target_core) {
                eprintln!("[!] Rollback failed: {}", e);
                std::process::exit(1);
            }
            println!("[OK] Rollback completed successfully!");
        } else {
            println!("[!] No previous binary found for rollback in '{}'.", target_dir.display());
        }
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&target_dir) {
        eprintln!("[!] Failed to create target directory: {}", e);
        std::process::exit(1);
    }

    let target_version = args
        .iter()
        .find(|a| a.starts_with("--version=") || a.starts_with("-v="))
        .and_then(|a| a.split('=').nth(1))
        .map(|s| s.to_string())
        .or_else(|| {
            println!("[*] Checking latest release from GitHub via HTTPS...");
            fetch_latest_release_tag().ok()
        })
        .unwrap_or_else(|| FALLBACK_VERSION.to_string());

    println!("[*] Target release: {}", target_version);

    let asset_name = get_distribution_asset_name();
    let temp_archive = std::env::temp_dir().join(format!("actx_dist_{}_{}", target_version, asset_name));

    if let Err(e) = download_release_asset(&target_version, asset_name, &temp_archive) {
        eprintln!("[!] Download failed: {}", e);
        std::process::exit(1);
    }

    let staging_dir = target_dir.join("actx_staging");
    if staging_dir.exists() {
        let _ = std::fs::remove_dir_all(&staging_dir);
    }

    if let Err(e) = extract_archive(&temp_archive, &staging_dir) {
        eprintln!("[!] Extraction failed: {}", e);
        let _ = std::fs::remove_file(&temp_archive);
        std::process::exit(1);
    }

    let _ = std::fs::remove_file(&temp_archive);

    if let Err(e) = finalize_staging_update(&target_dir, &staging_dir, Some(&target_version)) {
        eprintln!("[!] Installation finalization failed: {}", e);
        std::process::exit(1);
    }

    // Configure system PATH if not present
    configure_system_path(&target_dir);

    println!("\n[OK] AnyContext {} installed successfully!", target_version);
    println!("[>] Open a new terminal and run '{}' or 'actx --tui' to start.\n", get_shim_exe_name());
}

/// Executes a self-update returning a Result for UI-agnostic callers.
pub fn execute_standalone_update(base_dir: &Path, requested_version: Option<&str>) -> Result<String, String> {
    let target_version = match requested_version {
        Some(v) => {
            if v.starts_with('v') {
                v.to_string()
            } else {
                format!("v{}", v)
            }
        }
        None => fetch_latest_release_tag().map_err(|e| format!("Failed to resolve latest release: {}", e))?,
    };

    let asset_name = get_distribution_asset_name();
    let temp_archive = std::env::temp_dir().join(format!("actx_update_{}_{}", target_version, asset_name));

    download_release_asset(&target_version, asset_name, &temp_archive)
        .map_err(|e| format!("Download failed: {}", e))?;

    let staging_dir = base_dir.join("actx_staging");
    if staging_dir.exists() {
        let _ = std::fs::remove_dir_all(&staging_dir);
    }

    if let Err(e) = extract_archive(&temp_archive, &staging_dir) {
        let _ = std::fs::remove_file(&temp_archive);
        return Err(format!("Extraction failed: {}", e));
    }

    let _ = std::fs::remove_file(&temp_archive);

    finalize_staging_update(base_dir, &staging_dir, Some(&target_version))
        .map_err(|e| format!("Update finalization failed: {}", e))?;

    configure_system_path(base_dir);

    Ok(format!("AnyContext successfully updated to {} in {}", target_version, base_dir.display()))
}

/// Executes a self-update from the active installation directory with stdout progress.
pub fn run_standalone_update(base_dir: &Path, requested_version: Option<&str>) {
    println!("\n[*] Checking for AnyContext updates via HTTPS...");

    match execute_standalone_update(base_dir, requested_version) {
        Ok(msg) => {
            println!("\n[OK] {}", msg);
            println!("[>] Please restart 'actx' to launch the new version.\n");
        }
        Err(e) => {
            eprintln!("\n[!] {}", e);
        }
    }
}

pub fn read_version_from_pending_json(pending_flag: &Path) -> Option<String> {
    if !pending_flag.exists() {
        return None;
    }
    let content = std::fs::read_to_string(pending_flag).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    val.get("version").and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Checks and ensures that the bin directory is added to PATH on Windows with high precedence.
pub fn configure_system_path(bin_dir: &Path) {
    #[cfg(target_os = "windows")]
    {
        let bin_str = bin_dir.to_string_lossy().to_string();
        if let Ok(path_var) = env::var("PATH") {
            if !path_var.to_lowercase().contains(&bin_str.to_lowercase()) {
                println!("[*] Adding '{}' to User PATH environment variable...", bin_str);
                let _ = Command::new("powershell")
                    .args([
                        "-NoProfile",
                        "-Command",
                        &format!(
                            "[Environment]::SetEnvironmentVariable('Path', '{}' + ';' + [Environment]::GetEnvironmentVariable('Path', 'User'), 'User')",
                            bin_str
                        ),
                    ])
                    .status();
            }
        }
    }
}
