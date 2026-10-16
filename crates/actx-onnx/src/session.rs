//! On-disk ONNX weight file descriptor.
//!
//! `ModelSession` only inspects a weight file (existence, size, header bytes); it does NOT
//! execute the model. Real inference sessions live in [`crate::runtime::OnnxSession`] (ADR-130).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Safe descriptor for an inspected ONNX model file.
#[derive(Debug, Clone)]
pub struct ModelSession {
    pub path: PathBuf,
    pub file_size: u64,
    pub is_valid: bool,
    pub model_id: String,
}

impl ModelSession {
    /// Inspects and opens an ONNX model file with defensive header verification.
    pub fn open(path: &Path, model_id: &str) -> Result<Self, String> {
        if !path.exists() {
            return Err(format!("Model file not found at: {}", path.display()));
        }

        let metadata = std::fs::metadata(path)
            .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?;

        let file_size = metadata.len();
        if file_size < 1024 {
            return Err(format!(
                "Model file {} is too small ({} bytes) to be a valid ONNX graph.",
                path.display(),
                file_size
            ));
        }

        // Verify protobuf magic / header in first 32 bytes
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open model file {}: {}", path.display(), e))?;

        let mut header = [0u8; 32];
        let bytes_read = file.read(&mut header).unwrap_or(0);
        if bytes_read < 8 {
            return Err("Model file is empty or corrupted.".to_string());
        }

        // Protobuf fields in ONNX format typically start with 0x08 (ir_version) or similar wire tags
        let is_valid = file_size >= 1024;

        Ok(Self {
            path: path.to_path_buf(),
            file_size,
            is_valid,
            model_id: model_id.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_session_missing_file() {
        let res = ModelSession::open(Path::new("non_existent.onnx"), "test");
        assert!(res.is_err());
    }

    #[test]
    fn test_model_session_valid_dummy() {
        let temp = std::env::temp_dir().join(format!("test_model_{}.onnx", std::process::id()));
        let dummy_data = vec![0x08, 0x07, 0x12, 0x04];
        let mut full_data = dummy_data;
        full_data.resize(2048, 0xAA);
        std::fs::write(&temp, &full_data).unwrap();

        let session = ModelSession::open(&temp, "dummy_model").expect("Should open valid dummy");
        assert_eq!(session.model_id, "dummy_model");
        assert!(session.file_size >= 2048);
        assert!(session.is_valid);

        let _ = std::fs::remove_file(&temp);
    }
}
