pub mod atomic_swap;
pub mod downloader;
pub mod extractor;
pub mod paths;
pub mod validator;

use std::path::Path;
use std::process::Command;

use atomic_swap::finalize_staging_update;
use downloader::{download_release_asset, fetch_latest_release_tag};
use extractor::extract_archive;
pub use paths::{get_canonical_bin_dir, get_canonical_logs_dir, get_core_exe_name, get_distribution_asset_name, get_shim_exe_name, log_update_event};
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
    clean_lingering_old_files(&base_dir);
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            clean_lingering_old_files(parent);
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
            println!("[*] Checking for latest release...");
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
    log_update_event(
        "INFO",
        &format!(
            "execute_standalone_update initiated: requested={:?}, base_dir={}",
            requested_version,
            base_dir.display()
        ),
    );

    let target_version = match requested_version {
        Some(v) => {
            let clean = v.trim_start_matches('@');
            if clean.starts_with('v') || clean.starts_with('V') {
                clean.to_string()
            } else {
                format!("v{}", clean)
            }
        }
        None => match fetch_latest_release_tag() {
            Ok(tag) => tag,
            Err(e) => {
                let err_msg = format!("Failed to resolve latest release: {}", e);
                log_update_event("ERROR", &err_msg);
                return Err(err_msg);
            }
        },
    };

    log_update_event("INFO", &format!("Target release tag resolved: {}", target_version));

    let asset_name = get_distribution_asset_name();
    let temp_archive = std::env::temp_dir().join(format!("actx_update_{}_{}", target_version, asset_name));

    if let Err(e) = download_release_asset(&target_version, asset_name, &temp_archive) {
        if e.starts_with("ASSET_NOT_YET_AVAILABLE:") {
            let msg = format!(
                "Os pacotes executáveis para a versão {} ainda estão sendo preparados\n    pelos servidores de compilação (CI/CD) ou estão temporariamente indisponíveis.\n\n    💡 Dica: A compilação de novas releases leva alguns minutos após o lançamento.\n       Por favor, tente novamente em instantes com: actx --update\n       Acompanhe a publicação em: https://github.com/Levix-Digital/any-context-releases/releases/tag/{}",
                target_version, target_version
            );
            log_update_event("WARN", &msg);
            return Err(msg);
        }
        let err_msg = format!("Download failed: {}", e);
        log_update_event("ERROR", &err_msg);
        return Err(err_msg);
    }
    log_update_event("INFO", &format!("Downloaded release asset: {}", temp_archive.display()));

    let staging_dir = base_dir.join("actx_staging");
    if staging_dir.exists() {
        let _ = std::fs::remove_dir_all(&staging_dir);
    }

    if let Err(e) = extract_archive(&temp_archive, &staging_dir) {
        let _ = std::fs::remove_file(&temp_archive);
        let err_msg = format!("Extraction failed: {}", e);
        log_update_event("ERROR", &err_msg);
        return Err(err_msg);
    }

    let _ = std::fs::remove_file(&temp_archive);
    log_update_event("INFO", "Archive extracted successfully into staging directory.");

    if let Err(e) = finalize_staging_update(base_dir, &staging_dir, Some(&target_version)) {
        let err_msg = format!("Update finalization failed: {}", e);
        log_update_event("ERROR", &err_msg);
        return Err(err_msg);
    }
    log_update_event("INFO", "Staging update finalized (atomic swap completed).");

    configure_system_path(base_dir);
    heal_executing_and_shadowed_binaries(base_dir, &target_version);

    let ok_msg = format!("AnyContext successfully updated to {} in {}", target_version, base_dir.display());
    log_update_event("SUCCESS", &ok_msg);
    Ok(ok_msg)
}

/// Executes a self-update from the active installation directory with stdout progress.
pub fn run_standalone_update(base_dir: &Path, requested_version: Option<&str>) {
    println!("\n[*] Checking for AnyContext updates...");

    match execute_standalone_update(base_dir, requested_version) {
        Ok(msg) => {
            println!("\n[OK] {}", msg);
            println!("[>] Please restart 'actx' to launch the new version.\n");
        }
        Err(e) => {
            eprintln!("\n[!] {}\n", e);
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

/// Cleans any lingering .old backup files created during atomic self-updates.
pub fn clean_lingering_old_files(dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.contains(".old") || name.ends_with(".old") {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
    }
}

fn is_same_file_or_dir(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => {
            let sa = a.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
            let sb = b.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase();
            sa == sb
        }
    }
}

/// Safely replaces target binary with source binary, even if target is currently executing on Windows.
pub fn replace_target_binary(target: &Path, source: &Path) -> Result<(), std::io::Error> {
    if !source.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source binary not found: {}", source.display()),
        ));
    }

    #[cfg(target_os = "windows")]
    {
        // Try direct copy first
        if std::fs::copy(source, target).is_ok() {
            return Ok(());
        }

        // On Windows NT, an executing binary cannot be written directly (error 5: Access is denied),
        // but Windows allows renaming an active executing process.
        let unique = format!(
            "{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        let old_target = target.with_file_name(format!(
            "{}.old_{}",
            target.file_name().and_then(|n| n.to_str()).unwrap_or("actx"),
            unique
        ));

        std::fs::rename(target, &old_target)?;

        let mut copy_res = Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Initial copy attempt",
        ));
        for _ in 0..15 {
            match std::fs::copy(source, target) {
                Ok(_) => {
                    copy_res = Ok(());
                    break;
                }
                Err(e) => {
                    copy_res = Err(e);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        match copy_res {
            Ok(_) => {
                let _ = std::fs::remove_file(&old_target);
                Ok(())
            }
            Err(e) => {
                let _ = std::fs::rename(&old_target, target);
                Err(e)
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let parent = target.parent().unwrap_or_else(|| Path::new("."));
        let temp_target = parent.join(format!(".actx_tmp_{}", std::process::id()));
        std::fs::copy(source, &temp_target)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&temp_target, std::fs::Permissions::from_mode(0o755));
        }
        std::fs::rename(&temp_target, target)?;
        Ok(())
    }
}

/// Scans for and synchronizes the currently executing binary and any shadowed binaries found in PATH.
pub fn heal_executing_and_shadowed_binaries(base_dir: &Path, target_version: &str) {
    let shim_name = get_shim_exe_name();
    let source_shim = base_dir.join(shim_name);
    if !source_shim.exists() {
        return;
    }

    clean_lingering_old_files(base_dir);

    // 1. Check if the executing binary is outside base_dir (e.g. ~/.cargo/bin or custom path)
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            if !is_same_file_or_dir(parent, base_dir) {
                let file_name = current_exe.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name.to_lowercase().starts_with("actx") {
                    println!("[*] Synchronizing executing binary at '{}'...", current_exe.display());
                    match replace_target_binary(&current_exe, &source_shim) {
                        Ok(_) => {
                            let msg = format!(
                                "Updated executing binary at '{}' to {}!",
                                current_exe.display(),
                                target_version
                            );
                            println!("[OK] {}", msg);
                            log_update_event("INFO", &msg);
                        }
                        Err(e) => {
                            let warn_msg = format!(
                                "Could not update executing binary at '{}': {}",
                                current_exe.display(),
                                e
                            );
                            eprintln!("[!] Warning: {}", warn_msg);
                            log_update_event("WARN", &warn_msg);
                        }
                    }
                }
            }
        }
    }

    // 2. Scan PATH environment variable to detect and heal shadowed binaries
    if let Ok(path_var) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for dir_str in path_var.split(separator) {
            let dir = Path::new(dir_str.trim());
            if dir.as_os_str().is_empty() || is_same_file_or_dir(dir, base_dir) {
                continue;
            }
            clean_lingering_old_files(dir);
            let candidate = dir.join(shim_name);
            if candidate.is_file() {
                if let Ok(current_exe) = std::env::current_exe() {
                    if is_same_file_or_dir(&candidate, &current_exe) {
                        continue;
                    }
                }
                let needs_update = match (std::fs::metadata(&candidate), std::fs::metadata(&source_shim)) {
                    (Ok(m_cand), Ok(m_src)) => m_cand.len() != m_src.len(),
                    _ => true,
                };
                if needs_update {
                    println!("[*] Detected shadowed binary in PATH: '{}'. Synchronizing...", candidate.display());
                    match replace_target_binary(&candidate, &source_shim) {
                        Ok(_) => {
                            let msg = format!(
                                "Synchronized shadowed binary at '{}' to {}!",
                                candidate.display(),
                                target_version
                            );
                            println!("[OK] {}", msg);
                            log_update_event("INFO", &msg);
                        }
                        Err(e) => {
                            let warn_msg = format!(
                                "Found older actx binary at '{}' that could shadow AnyContext (error: {}). Please remove it or ensure '{}' takes precedence in PATH.",
                                candidate.display(),
                                e,
                                base_dir.display()
                            );
                            eprintln!("[!] Warning: {}", warn_msg);
                            log_update_event("WARN", &warn_msg);
                        }
                    }
                }
            }
        }
    }

    // 3. Explicitly inspect ~/.cargo/bin in case it is installed there without being directly in PATH
    if let Some(home) = dirs::home_dir() {
        let cargo_dir = home.join(".cargo").join("bin");
        clean_lingering_old_files(&cargo_dir);
        let cargo_candidate = cargo_dir.join(shim_name);
        if cargo_candidate.is_file() && !is_same_file_or_dir(&cargo_candidate, &source_shim) {
            let is_cur = std::env::current_exe().map(|c| is_same_file_or_dir(&c, &cargo_candidate)).unwrap_or(false);
            if !is_cur {
                let needs_update = match (std::fs::metadata(&cargo_candidate), std::fs::metadata(&source_shim)) {
                    (Ok(m_cand), Ok(m_src)) => m_cand.len() != m_src.len(),
                    _ => true,
                };
                if needs_update {
                    println!("[*] Synchronizing cargo binary at '{}'...", cargo_candidate.display());
                    match replace_target_binary(&cargo_candidate, &source_shim) {
                        Ok(_) => {
                            let msg = format!(
                                "Synchronized cargo binary at '{}' to {}!",
                                cargo_candidate.display(),
                                target_version
                            );
                            println!("[OK] {}", msg);
                            log_update_event("INFO", &msg);
                        }
                        Err(e) => {
                            let warn_msg = format!(
                                "Could not update cargo binary at '{}': {}",
                                cargo_candidate.display(),
                                e
                            );
                            eprintln!("[!] Warning: {}", warn_msg);
                            log_update_event("WARN", &warn_msg);
                        }
                    }
                }
            }
        }
    }
}

/// Checks and ensures that the bin directory is added to PATH on Windows with highest precedence.
pub fn configure_system_path(bin_dir: &Path) {
    #[cfg(target_os = "windows")]
    {
        let bin_str = bin_dir.to_string_lossy().to_string();
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    r#"$bin = '{}';
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User');
if (-not $userPath) {{ $userPath = '' }};
if ($userPath -notlike "$bin;*" -and $userPath -ne $bin) {{
    $clean = ($userPath -split ';' | Where-Object {{ $_ -ne '' -and $_.TrimEnd('\/').ToLower() -ne $bin.TrimEnd('\/').ToLower() }}) -join ';';
    $newPath = if ($clean) {{ "$bin;$clean" }} else {{ $bin }};
    [Environment]::SetEnvironmentVariable('Path', $newPath, 'User');
}}"#,
                    bin_str
                ),
            ])
            .status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_lingering_old_files() {
        let temp = std::env::temp_dir().join(format!("test_clean_old_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp);

        let old_file1 = temp.join("actx.exe.old");
        let old_file2 = temp.join("actx.old_12345");
        let valid_file = temp.join("actx.exe");

        std::fs::write(&old_file1, "old1").unwrap();
        std::fs::write(&old_file2, "old2").unwrap();
        std::fs::write(&valid_file, "valid").unwrap();

        clean_lingering_old_files(&temp);

        assert!(!old_file1.exists(), "old_file1 should be deleted");
        assert!(!old_file2.exists(), "old_file2 should be deleted");
        assert!(valid_file.exists(), "valid_file should be preserved");

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_replace_target_binary() {
        let temp = std::env::temp_dir().join(format!("test_replace_target_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp);

        let source = temp.join("source.exe");
        let target = temp.join("target.exe");

        std::fs::write(&source, "NEW_BINARY_CONTENT").unwrap();
        std::fs::write(&target, "OLD_BINARY_CONTENT").unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "OLD_BINARY_CONTENT");

        replace_target_binary(&target, &source).expect("replace_target_binary should succeed");

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "NEW_BINARY_CONTENT");

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_is_same_file_or_dir() {
        let temp = std::env::temp_dir();
        assert!(is_same_file_or_dir(&temp, &temp));
    }
}
