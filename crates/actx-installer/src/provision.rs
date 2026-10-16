//! Automatic model provisioning during install and update (ADR-130, D15).
//!
//! Downloads the files a profile needs (today: the cross-encoder re-ranker), verifying
//! size and SHA-256 before an atomic rename. Failures never abort the installation:
//! AnyContext keeps working with its RRF fallback and the user can repair later with
//! `actx download reranker`. Set `ACTX_SKIP_MODEL_DOWNLOAD=1` to skip provisioning.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};

use crate::hardware::ModelDownloadSpec;

/// Result of a provisioning pass.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProvisionReport {
    pub installed: Vec<String>,
    pub already_present: Vec<String>,
    pub failed: Vec<(String, String)>,
}

impl ProvisionReport {
    pub fn is_success(&self) -> bool {
        self.failed.is_empty()
    }
}

/// Converts a `/`-separated catalog path into a platform path under `base`.
pub fn resolve_model_path(base: &Path, rel: &str) -> PathBuf {
    rel.split('/').filter(|s| !s.is_empty()).fold(base.to_path_buf(), |acc, p| acc.join(p))
}

fn is_present(path: &Path, spec: &ModelDownloadSpec) -> bool {
    std::fs::metadata(path).map(|m| m.is_file() && m.len() == spec.size_bytes).unwrap_or(false)
}

/// True when provisioning is disabled by the environment.
pub fn provisioning_disabled() -> bool {
    std::env::var("ACTX_SKIP_MODEL_DOWNLOAD").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false)
}

/// Ensures every spec is present under `models_dir`, downloading and verifying missing files.
pub fn provision_models(models_dir: &Path, specs: &[ModelDownloadSpec], show_progress: bool) -> ProvisionReport {
    let mut report = ProvisionReport::default();
    for spec in specs {
        let dest = resolve_model_path(models_dir, spec.file_name);
        if is_present(&dest, spec) {
            report.already_present.push(spec.file_name.to_string());
            continue;
        }
        match download_verified(spec, &dest, show_progress) {
            Ok(()) => report.installed.push(spec.file_name.to_string()),
            Err(e) => report.failed.push((spec.file_name.to_string(), e)),
        }
    }
    report
}

fn download_verified(spec: &ModelDownloadSpec, dest: &Path, show_progress: bool) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    }
    let tmp = dest.with_file_name(format!(
        "{}.download.tmp",
        dest.file_name().and_then(|n| n.to_str()).unwrap_or("model")
    ));

    let response = ureq::get(spec.download_url)
        .set("User-Agent", concat!("AnyContext-Installer/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(1800))
        .call()
        .map_err(|e| format!("download failed: {e}"))?;

    let pb = if show_progress { ProgressBar::new(spec.size_bytes) } else { ProgressBar::hidden() };
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[Model] {msg} {bytes}/{total_bytes} [{bar:30.cyan/blue}] {bytes_per_sec}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_message(spec.file_name.to_string());

    let mut reader = response.into_reader();
    let mut out = File::create(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut written: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("stream error: {e}")
        })?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| format!("write error: {e}"))?;
        hasher.update(&buf[..n]);
        written += n as u64;
        pb.set_position(written);
    }
    out.flush().map_err(|e| format!("flush error: {e}"))?;
    drop(out);
    pb.finish_and_clear();

    let digest = format!("{:x}", hasher.finalize());
    if written != spec.size_bytes || !digest.eq_ignore_ascii_case(spec.sha256) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "integrity check failed (size {written}/{}, sha256 {digest})",
            spec.size_bytes
        ));
    }
    if dest.exists() {
        let _ = std::fs::remove_file(dest);
    }
    std::fs::rename(&tmp, dest).map_err(|e| format!("cannot finalize {}: {e}", dest.display()))?;
    Ok(())
}

/// Prints a concise, non-fatal summary of a provisioning pass.
pub fn print_report(report: &ProvisionReport) {
    for f in &report.installed {
        println!("[OK] Model installed and verified: {}", f);
    }
    if !report.already_present.is_empty() {
        println!("[OK] Models already present: {}", report.already_present.len());
    }
    for (f, e) in &report.failed {
        println!("[!] Could not install {} ({}). AnyContext keeps working with RRF; run `actx download reranker` later.", f, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_model_path_platform_separators() {
        let p = resolve_model_path(Path::new("base"), "mmarco-reranker/model.onnx");
        assert_eq!(p, Path::new("base").join("mmarco-reranker").join("model.onnx"));
    }

    #[test]
    fn test_present_files_are_not_downloaded() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ModelDownloadSpec {
            id: "fixture",
            file_name: "fixture/model.onnx",
            size_bytes: 4,
            download_url: "https://invalid.invalid/never-called",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
            description: "fixture",
        };
        let dest = resolve_model_path(dir.path(), spec.file_name);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::write(&dest, b"abcd").unwrap();
        let report = provision_models(dir.path(), &[spec], false);
        assert!(report.is_success());
        assert_eq!(report.already_present.len(), 1);
    }

    #[test]
    fn test_unreachable_source_is_reported_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ModelDownloadSpec {
            id: "fixture",
            file_name: "fixture/model.onnx",
            size_bytes: 4,
            download_url: "http://127.0.0.1:9/unreachable",
            sha256: "0000000000000000000000000000000000000000000000000000000000000000",
            description: "fixture",
        };
        let report = provision_models(dir.path(), &[spec], false);
        assert_eq!(report.failed.len(), 1);
        assert!(!resolve_model_path(dir.path(), "fixture/model.onnx").exists());
    }
}
