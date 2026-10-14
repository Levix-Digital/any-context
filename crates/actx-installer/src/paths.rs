use std::path::PathBuf;

/// Resolves the canonical installation bin directory cross-platform.
/// - Windows: %LOCALAPPDATA%\actx\bin (e.g. C:\Users\<User>\AppData\Local\actx\bin)
/// - macOS: ~/Library/Application Support/actx/bin (or ~/.local/bin)
/// - Linux: ~/.local/bin
pub fn get_canonical_bin_dir() -> PathBuf {
    if let Ok(override_dir) = std::env::var("ACTX_BIN_DIR") {
        if !override_dir.trim().is_empty() {
            return PathBuf::from(override_dir.trim());
        }
    }
    if let Ok(override_dir) = std::env::var("ACTX_UPDATE_DIR") {
        if !override_dir.trim().is_empty() {
            return PathBuf::from(override_dir.trim());
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(local_app_data) = dirs::data_local_dir() {
            return local_app_data.join("actx").join("bin");
        }
        if let Ok(app_data) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(app_data).join("actx").join("bin");
        }
        if let Some(home) = dirs::home_dir() {
            return home.join("AppData").join("Local").join("actx").join("bin");
        }
        PathBuf::from("C:\\actx\\bin")
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(data_dir) = dirs::data_dir() {
            return data_dir.join("actx").join("bin");
        }
        if let Some(home) = dirs::home_dir() {
            return home.join(".local").join("bin");
        }
        PathBuf::from("/usr/local/bin")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join(".local").join("bin");
        }
        PathBuf::from("/usr/local/bin")
    }
}

/// Returns the expected executable name for the native core engine.
pub fn get_core_exe_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "actx.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "actx"
    }
}

/// Returns the expected executable name for the native launcher shim.
pub fn get_shim_exe_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "actx.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "actx"
    }
}

/// Returns the expected distribution archive asset name for the current OS.
pub fn get_distribution_asset_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "actx-windows-x86_64.zip"
    }
    #[cfg(target_os = "macos")]
    {
        "actx-darwin-x86_64.tar.gz"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        "actx-linux-x86_64.tar.gz"
    }
}

/// Resolves the canonical logs directory cross-platform.
/// - Windows: %LOCALAPPDATA%\AnyContext\logs
/// - macOS: ~/Library/Logs/AnyContext
/// - Linux: ~/.local/share/any-context/logs
pub fn get_canonical_logs_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("AnyContext").join("logs");
        }
        if let Some(local_app_data) = dirs::data_local_dir() {
            return local_app_data.join("AnyContext").join("logs");
        }
        if let Some(home) = dirs::home_dir() {
            return home.join("AppData").join("Local").join("AnyContext").join("logs");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join("Library").join("Logs").join("AnyContext");
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join(".local").join("share").join("any-context").join("logs");
        }
    }
    std::env::temp_dir().join("AnyContext").join("logs")
}

/// Appends a structured log event to the persistent update.log file.
pub fn log_update_event(level: &str, message: &str) {
    let logs_dir = get_canonical_logs_dir();
    let _ = std::fs::create_dir_all(&logs_dir);
    let log_path = logs_dir.join("update.log");
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
        let ts = chrono::Utc::now().to_rfc3339();
        let _ = writeln!(file, "[{}] [{}] {}", ts, level, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_bin_dir_is_not_empty() {
        let dir = get_canonical_bin_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_core_and_shim_names() {
        let core = get_core_exe_name();
        let shim = get_shim_exe_name();
        assert!(core.contains("actx"));
        assert!(shim.contains("actx"));
    }

    #[test]
    fn test_distribution_asset_name() {
        let asset = get_distribution_asset_name();
        assert!(asset.starts_with("actx-"));
    }
}

