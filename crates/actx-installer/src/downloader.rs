use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;

pub const PRIMARY_REPO: &str = "Levix-Digital/any-context-releases";
pub const FALLBACK_REPO: &str = "Levix-Digital/any-context";

/// Fetches the latest published release tag from GitHub Releases via public HTTPS API.
pub fn fetch_latest_release_tag() -> Result<String, String> {
    for repo in [PRIMARY_REPO, FALLBACK_REPO] {
        let api_url = format!("https://api.github.com/repos/{}/releases/latest", repo);
        let resp = ureq::get(&api_url)
            .set("User-Agent", "AnyContext-Installer")
            .set("Accept", "application/vnd.github.v3+json")
            .timeout(std::time::Duration::from_secs(8))
            .call();

        if let Ok(response) = resp {
            if response.status() == 200 {
                if let Ok(json_val) = response.into_json::<Value>() {
                    if let Some(tag) = json_val.get("tag_name").and_then(|v| v.as_str()) {
                        let clean_tag = if tag.starts_with('v') {
                            tag.to_string()
                        } else {
                            format!("v{}", tag)
                        };
                        return Ok(clean_tag);
                    }
                }
            }
        }
    }

    Err("Could not retrieve latest release tag from GitHub (network unavailable or rate limited).".to_string())
}

/// Downloads a release asset directly from GitHub Releases via public HTTPS with real-time visual progress.
pub fn download_release_asset(
    tag: &str,
    asset_name: &str,
    dest_path: &Path,
) -> Result<(), String> {
    let clean_tag = if tag.starts_with('v') {
        tag.to_string()
    } else {
        format!("v{}", tag)
    };

    println!("[*] Downloading '{}' via HTTPS...", asset_name);

    if let Some(parent) = dest_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create download directory: {}", e))?;
    }

    let temp_download = dest_path.with_extension(format!("tmp_{}", std::process::id()));

    let repos = [PRIMARY_REPO, FALLBACK_REPO];
    let mut last_err = String::new();
    let mut successful_response = None;

    for repo in repos {
        let download_url = format!(
            "https://github.com/{}/releases/download/{}/{}",
            repo, clean_tag, asset_name
        );
        match ureq::get(&download_url)
            .set("User-Agent", "AnyContext-Installer")
            .timeout(std::time::Duration::from_secs(180))
            .call()
        {
            Ok(resp) if resp.status() == 200 => {
                successful_response = Some(resp);
                break;
            }
            Ok(resp) => {
                last_err = format!("Server returned HTTP {} from {}", resp.status(), repo);
            }
            Err(e) => {
                last_err = format!("HTTPS download request failed for {}: {}", download_url, e);
            }
        }
    }

    let response = match successful_response {
        Some(r) => r,
        None => return Err(last_err),
    };

    let total_size = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    let pb = if total_size > 0 {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[Download] {bytes}/{total_bytes} ({percent}%) [{bar:30.cyan/blue}] {bytes_per_sec}")
                .unwrap()
                .progress_chars("=>-")
        );
        Some(pb)
    } else {
        println!("[-] Downloading (size unknown)...");
        None
    };

    let mut reader = response.into_reader();
    let mut file = File::create(&temp_download)
        .map_err(|e| format!("Failed to create temporary file {}: {}", temp_download.display(), e))?;

    let mut buffer = [0u8; 1024 * 128]; // 128 KB buffer
    let mut downloaded: u64 = 0;

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .map_err(|e| format!("Download stream read error: {}", e))?;

        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .map_err(|e| format!("Failed to write to file: {}", e))?;

        downloaded += bytes_read as u64;
        if let Some(ref pb) = pb {
            pb.set_position(downloaded);
        }
    }

    file.flush()
        .map_err(|e| format!("Failed to flush downloaded file: {}", e))?;
    drop(file);

    if let Some(ref pb) = pb {
        pb.finish_with_message("Done");
    }

    // Atomic move of finished download to dest_path
    if dest_path.exists() {
        let _ = std::fs::remove_file(dest_path);
    }
    std::fs::rename(&temp_download, dest_path)
        .map_err(|e| format!("Failed to finalize downloaded file: {}", e))?;

    println!("[OK] Download finished: {} ({:.1} MB)", asset_name, downloaded as f64 / 1_048_576.0);
    Ok(())
}
