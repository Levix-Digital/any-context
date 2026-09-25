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

/// Returns the expected executable name for the heavy core engine.
pub fn get_core_exe_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "actx-core.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "actx-core"
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
        assert!(core.contains("actx-core"));
        assert!(shim.contains("actx"));
    }

    #[test]
    fn test_distribution_asset_name() {
        let asset = get_distribution_asset_name();
        assert!(asset.starts_with("actx-"));
    }
}

