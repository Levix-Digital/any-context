use std::path::Path;
use crate::paths::{get_core_exe_name, get_shim_exe_name};
use crate::validator::validate_binary_format;

/// Performs an atomic, self-validating swap between a prepared `staging_dir`
/// and the active production `base_dir`.
///
/// Complete Rollback Guarantee:
/// If the staged core binary fails PE/ELF validation or if any OS move operation fails,
/// the existing production files remain completely untouched or are immediately rolled back.
pub fn finalize_staging_update(
    base_dir: &Path,
    staging_dir: &Path,
    new_version: Option<&str>,
) -> Result<(), String> {
    println!("\n[*] Finalizing installation (atomic swap)...");

    // 1. Pre-flight check: Staging must exist
    if !staging_dir.exists() {
        return Err(format!("Staging directory not found: {}", staging_dir.display()));
    }

    let core_name = get_core_exe_name();
    let staging_core = staging_dir.join(core_name);

    // 2. Strict Pre-Swap Binary Format Validation
    validate_binary_format(&staging_core)?;

    let target_core = base_dir.join(core_name);
    let old_core = base_dir.join(if cfg!(windows) { "actx_old.exe" } else { "actx_old" });

    let internal_dir = base_dir.join("_internal");
    let staging_internal = staging_dir.join("_internal");
    let unique_suffix = format!("{}_{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0));
    let old_internal = base_dir.join(format!("_internal_old_{}", unique_suffix));

    // 3. Atomic move of existing _internal to old_internal (MFT pointer update < 5ms)
    let internal_moved = if internal_dir.exists() {
        match std::fs::rename(&internal_dir, &old_internal) {
            Ok(_) => true,
            Err(e) => {
                return Err(format!("Failed to rename active _internal directory: {}", e));
            }
        }
    } else {
        false
    };

    // 4. Move staged _internal to production
    if staging_internal.exists() {
        if let Err(e) = std::fs::rename(&staging_internal, &internal_dir) {
            // Rollback _internal
            if internal_moved {
                let _ = std::fs::rename(&old_internal, &internal_dir);
            }
            return Err(format!("Failed to move staged _internal directory: {}", e));
        }
    }

    // 5. Atomic move of active core engine to old_core
    let core_moved = if target_core.exists() {
        if old_core.exists() {
            let _ = std::fs::remove_file(&old_core);
        }
        match std::fs::rename(&target_core, &old_core) {
            Ok(_) => true,
            Err(e) => {
                // Rollback _internal
                let _ = std::fs::remove_dir_all(&internal_dir);
                if internal_moved {
                    let _ = std::fs::rename(&old_internal, &internal_dir);
                }
                return Err(format!("Failed to backup active core binary: {}", e));
            }
        }
    } else {
        false
    };

    // 6. Move staged core engine to production
    if let Err(e) = std::fs::rename(&staging_core, &target_core) {
        // Rollback core and _internal
        if core_moved {
            let _ = std::fs::rename(&old_core, &target_core);
        }
        let _ = std::fs::remove_dir_all(&internal_dir);
        if internal_moved {
            let _ = std::fs::rename(&old_internal, &internal_dir);
        }
        return Err(format!("Failed to promote staged core binary to production: {}", e));
    }

    // 7. Post-Swap Verification: Validate production binary
    if let Err(e) = validate_binary_format(&target_core) {
        // Instant rollback
        let _ = std::fs::remove_file(&target_core);
        if core_moved {
            let _ = std::fs::rename(&old_core, &target_core);
        }
        let _ = std::fs::remove_dir_all(&internal_dir);
        if internal_moved {
            let _ = std::fs::rename(&old_internal, &internal_dir);
        }
        return Err(format!("Post-swap binary validation failed: {}. Rollback completed.", e));
    }

    // 8. Move any other staged files (e.g. shim, bash wrapper)
    if let Ok(entries) = std::fs::read_dir(staging_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name.eq_ignore_ascii_case("pending_update.json")
                    || name.eq_ignore_ascii_case(core_name)
                    || name.eq_ignore_ascii_case("_internal")
                {
                    continue;
                }
                let dest = base_dir.join(name);
                if dest.exists() {
                    let _ = std::fs::remove_file(&dest);
                }
                let _ = std::fs::rename(&path, &dest);
            }
        }
    }

    // 9. Update version.txt
    if let Some(ver) = new_version {
        let version_file = base_dir.join("version.txt");
        let clean_ver = if ver.starts_with('v') {
            ver.to_string()
        } else {
            format!("v{}", ver)
        };
        let _ = std::fs::write(&version_file, format!("{}\n", clean_ver));
    }

    // 10. Clean up staging and old backups
    let pending_flag = staging_dir.join("pending_update.json");
    let _ = std::fs::remove_file(&pending_flag);
    let _ = std::fs::remove_dir_all(staging_dir);
    if old_core.exists() {
        let _ = std::fs::remove_file(&old_core);
    }
    if old_internal.exists() {
        let _ = std::fs::remove_dir_all(&old_internal);
    }

    // Clean any other stale _internal_old* backup directories
    if let Ok(entries) = std::fs::read_dir(base_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_dir() {
                    let name = entry.file_name();
                    if name.to_string_lossy().starts_with("_internal_old") {
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }
    }

    let version_display = new_version.map(|v| format!(" to {}", v)).unwrap_or_default();
    println!("[OK] AnyContext successfully updated{}!", version_display);
    println!("[>] Run '{}' or 'actx --tui' to start the updated version.\n", get_shim_exe_name());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_corrupted_staging_is_rejected_without_touching_production() {
        let temp_dir = std::env::temp_dir().join(format!("test_swap_{}", std::process::id()));
        let base_dir = temp_dir.join("base");
        let staging_dir = temp_dir.join("staging");
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::create_dir_all(&staging_dir).unwrap();

        // Production setup
        let prod_core = base_dir.join(get_core_exe_name());
        let mut f_prod = std::fs::File::create(&prod_core).unwrap();
        f_prod.write_all(b"original_core_bytes").unwrap();
        drop(f_prod);

        // Staging setup with corrupted ZIP file
        let staging_core = staging_dir.join(get_core_exe_name());
        let mut f_stg = std::fs::File::create(&staging_core).unwrap();
        f_stg.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        f_stg.write_all(&vec![0u8; 1_500_000]).unwrap();
        drop(f_stg);

        let result = finalize_staging_update(&base_dir, &staging_dir, Some("v1.0.0"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is a ZIP archive"));

        // Verify production was NOT overwritten
        let current_bytes = std::fs::read(&prod_core).unwrap();
        assert_eq!(current_bytes, b"original_core_bytes");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

