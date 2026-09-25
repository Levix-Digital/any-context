use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Validates that the file at `path` is an authentic, non-corrupted native executable
/// matching the current operating system's binary format specification.
///
/// Specifically guards against:
/// 1. Files that are actually ZIP or TAR.GZ archives erroneously named or swapped as executables.
/// 2. Truncated or empty 0-byte downloads.
/// 3. Incompatible binary formats (e.g. Linux ELF on Windows or vice-versa).
pub fn validate_binary_format(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Binary file does not exist: {}", path.display()));
    }

    let metadata = std::fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?;

    let size = metadata.len();
    if size < 1_000_000 {
        return Err(format!(
            "Binary file is too small ({} bytes). Expected at least 1MB.",
            size
        ));
    }

    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open file {}: {}", path.display(), e))?;

    let mut header = [0u8; 4];
    file.read_exact(&mut header)
        .map_err(|e| format!("Failed to read header from {}: {}", path.display(), e))?;

    // Universal Archive Detection Guard
    if header == [0x50, 0x4B, 0x03, 0x04] || header == [0x50, 0x4B, 0x05, 0x06] {
        return Err(format!(
            "FATAL: File '{}' is a ZIP archive, not an executable binary (magic bytes: PK..). Swap aborted.",
            path.display()
        ));
    }
    if header[0..2] == [0x1F, 0x8B] {
        return Err(format!(
            "FATAL: File '{}' is a GZIP archive, not an executable binary. Swap aborted.",
            path.display()
        ));
    }

    #[cfg(target_os = "windows")]
    {
        // Windows Portable Executable (PE) magic number: "MZ" (0x4D, 0x5A)
        if header[0..2] != [0x4D, 0x5A] {
            return Err(format!(
                "Invalid Windows binary: expected 'MZ' header (0x4D, 0x5A), got [{:02X}, {:02X}] in '{}'",
                header[0], header[1], path.display()
            ));
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux ELF magic number: 0x7F 'E' 'L' 'F' (0x7F, 0x45, 0x4C, 0x46)
        if header != [0x7F, 0x45, 0x4C, 0x46] {
            return Err(format!(
                "Invalid Linux binary: expected ELF header (0x7F, 0x45, 0x4C, 0x46), got [{:02X}, {:02X}, {:02X}, {:02X}] in '{}'",
                header[0], header[1], header[2], header[3], path.display()
            ));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS Mach-O magic numbers:
        // 0xFEEDFACE (32-bit), 0xFEEDFACF (64-bit), 0xCAFEBABE (Universal Binary), and little-endian variants
        let is_macho = header == [0xFE, 0xED, 0xFA, 0xCE]
            || header == [0xFE, 0xED, 0xFA, 0xCF]
            || header == [0xCE, 0xFA, 0xED, 0xFE]
            || header == [0xCF, 0xFA, 0xED, 0xFE]
            || header == [0xCA, 0xFE, 0xBA, 0xBE]
            || header == [0xBE, 0xBA, 0xFE, 0xCA];

        if !is_macho {
            return Err(format!(
                "Invalid macOS Mach-O binary in '{}': header [{:02X}, {:02X}, {:02X}, {:02X}] does not match Mach-O magic signature.",
                path.display(), header[0], header[1], header[2], header[3]
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_zip_header_is_strictly_rejected() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_fake_exe.exe");

        // Write 1.5MB with ZIP magic bytes
        let mut f = File::create(&test_file).unwrap();
        f.write_all(&[0x50, 0x4B, 0x03, 0x04]).unwrap();
        let padding = vec![0u8; 1_500_000];
        f.write_all(&padding).unwrap();
        drop(f);

        let result = validate_binary_format(&test_file);
        let _ = std::fs::remove_file(&test_file);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("is a ZIP archive"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_pe_header_is_accepted() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_valid_pe.exe");

        let mut f = File::create(&test_file).unwrap();
        f.write_all(&[0x4D, 0x5A, 0x90, 0x00]).unwrap();
        let padding = vec![0u8; 1_500_000];
        f.write_all(&padding).unwrap();
        drop(f);

        let result = validate_binary_format(&test_file);
        let _ = std::fs::remove_file(&test_file);

        assert!(result.is_ok());
    }
}
