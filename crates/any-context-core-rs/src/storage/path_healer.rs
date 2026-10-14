//! Smart Path Healing Engine for AnyContext.
//!
//! Heuristically resolves and self-heals broken, misconfigured, or slightly shifted
//! filesystem paths (e.g. redundant/missing `Documentos/`, `Documents/`, `OneDrive/`,
//! slash direction discrepancies, and volume mounts like Google Drive `G:/`).

use std::path::{Path, PathBuf};

/// Attempts to heuristically heal a non-existent directory path.
///
/// Handles common path discrepancies such as:
/// 1. Slashes / Backslashes normalization
/// 2. Redundant "Documentos/" or "Documents/" (e.g. `G:/My Drive/Documentos/Levix Digital` -> `G:/My Drive/Levix Digital`)
/// 3. Missing "Documentos/" or "Documents/"
/// 4. Redundant "OneDrive/" or "OneDrive - .../"
/// 5. Finding matching leaf folder in parent or drive root
pub fn try_heal_path(p: &Path) -> Option<PathBuf> {
    if p.exists() {
        return Some(p.to_path_buf());
    }

    let raw_str = p.to_string_lossy();
    // 1. Slashes normalization
    let normalized_str = raw_str.replace('\\', "/");
    let norm_path = PathBuf::from(&normalized_str);
    if norm_path.exists() {
        return Some(norm_path);
    }

    // 2. Attempt: Remove "Documentos/" or "Documents/"
    let without_doc = normalized_str
        .replace("/Documentos/", "/")
        .replace("/Documents/", "/");
    let candidate_without_doc = PathBuf::from(&without_doc);
    if candidate_without_doc.exists() && candidate_without_doc != norm_path {
        return Some(candidate_without_doc);
    }

    // 3. Attempt: Add "Documentos/" or "Documents/" under "My Drive"
    if normalized_str.contains("/My Drive/") && !normalized_str.contains("/My Drive/Documentos/") {
        let with_doc = normalized_str.replace("/My Drive/", "/My Drive/Documentos/");
        let candidate_with_doc = PathBuf::from(&with_doc);
        if candidate_with_doc.exists() {
            return Some(candidate_with_doc);
        }
    }

    // 4. Attempt: Remove "OneDrive/"
    if normalized_str.contains("/OneDrive/") {
        let without_onedrive = normalized_str.replace("/OneDrive/", "/");
        let candidate = PathBuf::from(&without_onedrive);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 5. Attempt: Parent tree search for leaf directory
    if let Some(file_name) = p.file_name() {
        if let Some(parent) = p.parent() {
            if let Some(grandparent) = parent.parent() {
                let candidate = grandparent.join(file_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_heal_existing_path() {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(try_heal_path(&cwd), Some(cwd));
    }

    #[test]
    fn test_try_heal_non_existent_fake_path() {
        let fake = Path::new("Z:/NonExistentVolume/NoSuchDirectory12345");
        assert_eq!(try_heal_path(fake), None);
    }

    #[test]
    fn test_try_heal_redundant_documentos_temp() {
        let temp_dir = std::env::temp_dir().join(format!("test_heal_{}", std::process::id()));
        let real_target = temp_dir.join("Levix Digital").join("AnyContext");
        let _ = std::fs::create_dir_all(&real_target);

        // Simulated broken path with "Documentos" injected
        let broken = temp_dir.join("Documentos").join("Levix Digital").join("AnyContext");
        assert!(!broken.exists());

        let healed = try_heal_path(&broken);
        assert!(healed.is_some());
        assert_eq!(healed.unwrap(), real_target);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
