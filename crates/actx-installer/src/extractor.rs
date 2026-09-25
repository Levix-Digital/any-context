use std::fs::File;
use std::path::Path;

/// Unpacks a distribution package (.zip or .tar.gz) into `dest_dir`.
/// Guards against Zip-Slip path traversal vulnerabilities and sets executable permissions on Unix.
pub fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    println!("[*] Extracting package contents to '{}'...", dest_dir.display());
    std::fs::create_dir_all(dest_dir)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;

    let file_name = archive_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    if file_name.ends_with(".zip") {
        extract_zip(archive_path, dest_dir)
    } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        extract_tar_gz(archive_path, dest_dir)
    } else {
        Err(format!(
            "Unsupported archive format for '{}'. Expected .zip or .tar.gz.",
            archive_path.display()
        ))
    }
}

fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open zip archive {}: {}", archive_path.display(), e))?;

    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Corrupted or invalid zip archive {}: {}", archive_path.display(), e))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry #{}: {}", i, e))?;

        let enclosed_name = match entry.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue, // Skip malicious or path traversal entries
        };

        let out_path = dest_dir.join(enclosed_name);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory {}: {}", out_path.display(), e))?;
        } else {
            if let Some(parent) = out_path.parent() {
                if !parent.exists() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent dir: {}", e))?;
                }
            }

            let mut outfile = File::create(&out_path)
                .map_err(|e| format!("Failed to create output file {}: {}", out_path.display(), e))?;

            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to write file {}: {}", out_path.display(), e))?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ = std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(mode));
                }
            }
        }
    }

    println!("[OK] Archive extraction completed successfully.");
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("Failed to open tar.gz archive {}: {}", archive_path.display(), e))?;

    let tar = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(tar);

    archive
        .unpack(dest_dir)
        .map_err(|e| format!("Failed to unpack tar.gz to {}: {}", dest_dir.display(), e))?;

    println!("[OK] Archive extraction completed successfully.");
    Ok(())
}
