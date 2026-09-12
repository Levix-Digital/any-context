use std::path::Path;
use crate::models::ChunkPayload;

/// High-performance native Image and Diagram Chunker.
/// Implements Nível 2 and Nível 3 of the Smart Cascade:
/// - Decodes PNG, JPEG, and WebP metadata in pure native Rust (zero Python dependencies).
/// - Nível 2 (OCR Gate): Executes native Tesseract OCR if available on PATH or system directories.
/// - If OCR extracts meaningful text (>= 30 chars): emits OCR text chunk with breadcrumb:
///   `// Context: <file> > Image (OCR Scan)`
/// - If OCR is not available or returns < 30 chars (visual diagrams, architecture, charts):
///   emits structured visual specification with breadcrumb:
///   `// Context: <file> > Visual Diagram [Resolution: WxH, Format: ...]`
///   and marks `content_type: "visual_diagram"` for Nível 3 (Vision LLM hook).
#[derive(Debug, Clone)]
pub struct ImageChunker {
    pub max_chunk_chars: usize,
    pub min_ocr_chars: usize,
}

impl Default for ImageChunker {
    fn default() -> Self {
        Self::new(1800)
    }
}

impl ImageChunker {
    pub fn new(max_chunk_chars: usize) -> Self {
        Self {
            max_chunk_chars,
            min_ocr_chars: 30,
        }
    }

    pub fn supports_extension(&self, ext: &str) -> bool {
        matches!(
            ext.to_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "webp"
        )
    }

    /// Reads and chunks an image file directly from the filesystem.
    pub fn chunk_file(&self, file_path: &str) -> Result<Vec<ChunkPayload>, String> {
        let bytes = std::fs::read(file_path).map_err(|e| {
            format!("Failed to read image file '{}': {}", file_path, e)
        })?;
        self.chunk_bytes(file_path, &bytes)
    }

    /// Chunks raw byte content of an image file.
    pub fn chunk_bytes(&self, file_path: &str, bytes: &[u8]) -> Result<Vec<ChunkPayload>, String> {
        let p = Path::new(file_path);
        let file_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png")
            .to_string();
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
            .to_uppercase();

        // 1. Decode image metadata in pure native Rust
        let (width, height, color_desc, format_name) = match image::load_from_memory(bytes) {
            Ok(img) => {
                let w = img.width();
                let h = img.height();
                let c = format!("{:?}", img.color());
                (w, h, c, ext)
            }
            Err(_) => {
                // Fallback for raw byte buffers or headers that could not be fully decoded
                (0, 0, "Unknown".to_string(), ext)
            }
        };

        // 2. Nível 2: Attempt Native OCR Gate if file exists on disk
        let mut ocr_text: Option<String> = None;
        if std::path::Path::new(file_path).exists() {
            ocr_text = self.run_native_ocr(file_path);
        }

        if let Some(text) = ocr_text {
            let clean = text.trim();
            if clean.len() >= self.min_ocr_chars {
                let header = format!("{} > Image (OCR Scan)", file_name);
                let full_text = format!(
                    "// Context: {}\n\n### Image Text Content (OCR Scan)\n- **Source File**: {}\n- **Resolution**: {}x{} px\n- **Format**: {}\n\n---\n{}",
                    header, file_name, width, height, format_name, clean
                );
                let line_count = full_text.lines().count().max(1);
                return Ok(vec![ChunkPayload {
                    id: format!("{}_img_ocr_0", file_name),
                    text: full_text,
                    file_name: file_name.clone(),
                    file_path: file_path.to_string(),
                    header_path: Some(header),
                    start_line: 1,
                    end_line: line_count,
                    content_type: "image_ocr".to_string(),
                    chunk_index: 0,
                }]);
            }
        }

        // 3. Nível 3: Visual Diagram & Architecture Specification (Vision Gate)
        let aspect_ratio = if height > 0 {
            let ratio = (width as f64) / (height as f64);
            if (ratio - 1.777).abs() < 0.1 {
                "16:9 (Widescreen)".to_string()
            } else if (ratio - 1.333).abs() < 0.1 {
                "4:3 (Standard)".to_string()
            } else if (ratio - 1.0).abs() < 0.05 {
                "1:1 (Square)".to_string()
            } else {
                format!("{:.2}:1", ratio)
            }
        } else {
            "N/A".to_string()
        };

        let header = format!("{} > Visual Diagram [Resolution: {}x{} px]", file_name, width, height);
        let visual_text = format!(
            "// Context: {}\n\n### Visual Image & Diagram Specification\n- **File**: {}\n- **Resolution**: {} x {} px\n- **Aspect Ratio**: {}\n- **Color Mode**: {}\n- **Format**: {}\n- **Type**: Visual Diagram / Technical Image\n\n> Note: This image contains visual diagrams, charts, UI mockups or schematics indexed for multimodal knowledge grounding.",
            header, file_name, width, height, aspect_ratio, color_desc, format_name
        );

        let line_count = visual_text.lines().count().max(1);
        Ok(vec![ChunkPayload {
            id: format!("{}_img_vis_0", file_name),
            text: visual_text,
            file_name: file_name.clone(),
            file_path: file_path.to_string(),
            header_path: Some(header),
            start_line: 1,
            end_line: line_count,
            content_type: "visual_diagram".to_string(),
            chunk_index: 0,
        }])
    }

    /// Invokes native OCR binary (Tesseract) directly from the operating system or portable bundle.
    fn run_native_ocr(&self, file_path: &str) -> Option<String> {
        let candidates = resolve_tesseract_candidates();

        for (bin, tessdata_prefix) in &candidates {
            // First try with bilingual por+eng
            let mut cmd = std::process::Command::new(bin);
            cmd.arg(file_path)
                .arg("stdout")
                .arg("-l")
                .arg("por+eng")
                .arg("--oem")
                .arg("1");
            if let Some(ref prefix) = tessdata_prefix {
                cmd.env("TESSDATA_PREFIX", prefix);
            }
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }

            // Fallback try with default language
            let mut cmd_fallback = std::process::Command::new(bin);
            cmd_fallback.arg(file_path).arg("stdout");
            if let Some(ref prefix) = tessdata_prefix {
                cmd_fallback.env("TESSDATA_PREFIX", prefix);
            }
            if let Ok(output) = cmd_fallback.output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !text.is_empty() {
                        return Some(text);
                    }
                }
            }
        }

        None
    }
}

/// Resolves potential locations of the Tesseract binary, prioritizing portable local installs.
fn resolve_tesseract_candidates() -> Vec<(String, Option<String>)> {
    let mut list = Vec::new();

    // 1. Check %LOCALAPPDATA%\actx\bin\tesseract\tesseract.exe
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let p = std::path::PathBuf::from(&local_app_data)
            .join("actx")
            .join("bin")
            .join("tesseract")
            .join("tesseract.exe");
        if p.exists() {
            let tessdata = p.parent().unwrap().join("tessdata");
            let prefix = if tessdata.exists() {
                Some(tessdata.to_string_lossy().to_string())
            } else {
                None
            };
            list.push((p.to_string_lossy().to_string(), prefix));
        }
    }

    // 2. Check relative to current running executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let p1 = parent.join("tesseract").join("tesseract.exe");
            if p1.exists() {
                let tessdata = p1.parent().unwrap().join("tessdata");
                let prefix = if tessdata.exists() {
                    Some(tessdata.to_string_lossy().to_string())
                } else {
                    None
                };
                list.push((p1.to_string_lossy().to_string(), prefix));
            }
            let p2 = parent.join("_internal").join("tesseract").join("tesseract.exe");
            if p2.exists() {
                let tessdata = p2.parent().unwrap().join("tessdata");
                let prefix = if tessdata.exists() {
                    Some(tessdata.to_string_lossy().to_string())
                } else {
                    None
                };
                list.push((p2.to_string_lossy().to_string(), prefix));
            }
        }
    }

    // 3. Check ~/.local/share/actx/bin/tesseract on Linux/macOS
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::PathBuf::from(&home)
            .join(".local")
            .join("share")
            .join("actx")
            .join("bin")
            .join("tesseract");
        if p.exists() {
            list.push((p.to_string_lossy().to_string(), None));
        }
    }

    // 4. System PATH and standard system installations
    list.push(("tesseract".to_string(), None));
    list.push(("C:\\Program Files\\Tesseract-OCR\\tesseract.exe".to_string(), None));
    list.push(("C:\\Program Files (x86)\\Tesseract-OCR\\tesseract.exe".to_string(), None));
    list.push(("/usr/bin/tesseract".to_string(), None));
    list.push(("/usr/local/bin/tesseract".to_string(), None));
    list.push(("/opt/homebrew/bin/tesseract".to_string(), None));

    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_image_extensions() {
        let chunker = ImageChunker::new(500);
        assert!(chunker.supports_extension("png"));
        assert!(chunker.supports_extension("PNG"));
        assert!(chunker.supports_extension("jpg"));
        assert!(chunker.supports_extension("jpeg"));
        assert!(chunker.supports_extension("webp"));
        assert!(!chunker.supports_extension("pdf"));
        assert!(!chunker.supports_extension("txt"));
    }

    #[test]
    fn test_visual_diagram_metadata_generation() {
        let chunker = ImageChunker::new(500);
        // Test with a dummy byte buffer that simulates an image
        let chunks = chunker.chunk_bytes("architecture_diagram.png", b"fake image bytes").unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content_type, "visual_diagram");
        assert!(chunks[0].header_path.as_ref().unwrap().contains("architecture_diagram.png"));
        assert!(chunks[0].text.contains("Visual Image & Diagram Specification"));
    }
}
