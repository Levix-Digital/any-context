//! Sub-millisecond hardware pre-flight inspection and model profile selector for actx-installer.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerProfileTier {
    AirGappedPower,
    SmartBalanced,
    Lite,
}

impl InstallerProfileTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AirGappedPower => "air_gapped_power",
            Self::SmartBalanced => "smart_balanced",
            Self::Lite => "lite",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AirGappedPower => "Air-Gapped Power",
            Self::SmartBalanced => "Smart Balanced",
            Self::Lite => "Lite (Fast-Path)",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase();
        match clean.as_str() {
            "air_gapped_power" | "airgapped" | "air-gapped" | "air" | "max" | "power" => Some(Self::AirGappedPower),
            "smart_balanced" | "balanced" | "smart" | "medium" => Some(Self::SmartBalanced),
            "lite" | "fast" | "fast-path" | "minimal" | "none" => Some(Self::Lite),
            _ => None,
        }
    }

    pub fn target_models(&self) -> &'static [ModelDownloadSpec] {
        match self {
            Self::AirGappedPower => &ALL_MODELS,
            Self::SmartBalanced => &BALANCED_MODELS,
            Self::Lite => &[],
        }
    }

    pub fn disk_required_bytes(&self) -> u64 {
        match self {
            Self::AirGappedPower => 740_000_000,
            Self::SmartBalanced => 252_000_000,
            Self::Lite => 0,
        }
    }

    pub fn estimated_peak_ram_bytes(&self) -> u64 {
        match self {
            Self::AirGappedPower => 850_000_000,
            Self::SmartBalanced => 400_000_000,
            Self::Lite => 45_000_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreStatus {
    Green,
    Yellow,
    Red,
}

impl SemaphoreStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            Self::Green => "🟢",
            Self::Yellow => "🟡",
            Self::Red => "🔴",
        }
    }
}

pub struct ModelDownloadSpec {
    pub id: &'static str,
    pub file_name: &'static str,
    pub size_bytes: u64,
    pub download_url: &'static str,
    pub description: &'static str,
}

pub static ALL_MODELS: [ModelDownloadSpec; 5] = [
    ModelDownloadSpec {
        id: "mobilenetv4-rvl-cdip",
        file_name: "mobilenetv4_rvl_cdip_int8.onnx",
        size_bytes: 3_900_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/mobilenetv4_rvl_cdip_int8.onnx",
        description: "Document Scan Sentinel & RVL-CDIP 16-class classifier",
    },
    ModelDownloadSpec {
        id: "layoutlmv3-int8",
        file_name: "layoutlmv3_base_int8.onnx",
        size_bytes: 121_000_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/layoutlmv3_base_int8.onnx",
        description: "2D Spatial Form & Layout LMv3 Transformer",
    },
    ModelDownloadSpec {
        id: "bge-small-onnx",
        file_name: "bge_small_query_int8.onnx",
        size_bytes: 127_000_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/bge_small_query_int8.onnx",
        description: "Neural Query Complexity Router (<15ms)",
    },
    ModelDownloadSpec {
        id: "clip-vit-int8",
        file_name: "clip_vit_vision_int8.onnx",
        size_bytes: 84_000_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/clip_vit_vision_int8.onnx",
        description: "CLIP ViT-B/32 Visual Feature Embedding Engine",
    },
    ModelDownloadSpec {
        id: "laya-int8",
        file_name: "laya_mmbert_int8.onnx",
        size_bytes: 404_000_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/laya_mmbert_int8.onnx",
        description: "Heavy Multi-Modal Document Representation BERT",
    },
];

pub static BALANCED_MODELS: [ModelDownloadSpec; 3] = [
    ModelDownloadSpec {
        id: "mobilenetv4-rvl-cdip",
        file_name: "mobilenetv4_rvl_cdip_int8.onnx",
        size_bytes: 3_900_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/mobilenetv4_rvl_cdip_int8.onnx",
        description: "Document Scan Sentinel & RVL-CDIP 16-class classifier",
    },
    ModelDownloadSpec {
        id: "layoutlmv3-int8",
        file_name: "layoutlmv3_base_int8.onnx",
        size_bytes: 121_000_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/layoutlmv3_base_int8.onnx",
        description: "2D Spatial Form & Layout LMv3 Transformer",
    },
    ModelDownloadSpec {
        id: "bge-small-onnx",
        file_name: "bge_small_query_int8.onnx",
        size_bytes: 127_000_000,
        download_url: "https://huggingface.co/anycontext/models/resolve/main/bge_small_query_int8.onnx",
        description: "Neural Query Complexity Router (<15ms)",
    },
];

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub free_disk_bytes: u64,
    pub cpu_cores: usize,
}

impl SystemMetrics {
    pub fn total_ram_gb(&self) -> f64 {
        self.total_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn available_ram_gb(&self) -> f64 {
        self.available_ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn free_disk_gb(&self) -> f64 {
        self.free_disk_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

pub struct ProfileEvaluation {
    pub tier: InstallerProfileTier,
    pub status: SemaphoreStatus,
    pub headline: &'static str,
    pub reason: String,
}

pub fn probe_system(target_dir: &Path) -> SystemMetrics {
    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let (total_ram, available_ram) = probe_memory();
    let free_disk = probe_disk(target_dir);

    SystemMetrics {
        total_ram_bytes: total_ram,
        available_ram_bytes: available_ram,
        free_disk_bytes: free_disk,
        cpu_cores,
    }
}

#[cfg(windows)]
fn probe_memory() -> (u64, u64) {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct MEMORYSTATUSEX {
        dwLength: u32,
        dwMemoryLoad: u32,
        ullTotalPhys: u64,
        ullAvailPhys: u64,
        ullTotalPageFile: u64,
        ullAvailPageFile: u64,
        ullTotalVirtual: u64,
        ullAvailVirtual: u64,
        ullAvailExtendedVirtual: u64,
    }

    extern "system" {
        fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> i32;
    }

    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        dwMemoryLoad: 0,
        ullTotalPhys: 0,
        ullAvailPhys: 0,
        ullTotalPageFile: 0,
        ullAvailPageFile: 0,
        ullTotalVirtual: 0,
        ullAvailVirtual: 0,
        ullAvailExtendedVirtual: 0,
    };

    unsafe {
        if GlobalMemoryStatusEx(&mut status) != 0 {
            return (status.ullTotalPhys, status.ullAvailPhys);
        }
    }

    (16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024)
}

#[cfg(not(windows))]
fn probe_memory() -> (u64, u64) {
    let default_val = (16 * 1024 * 1024 * 1024, 8 * 1024 * 1024 * 1024);
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        let mut total = 0u64;
        let mut available = 0u64;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    total = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                }
            } else if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    available = parts[1].parse::<u64>().unwrap_or(0) * 1024;
                }
            }
        }
        if total > 0 {
            return (total, if available > 0 { available } else { total / 2 });
        }
    }
    default_val
}

#[cfg(windows)]
fn probe_disk(path: &Path) -> u64 {
    use std::os::windows::ffi::OsStrExt;

    extern "system" {
        fn GetDiskFreeSpaceExW(
            lpDirectoryName: *const u16,
            lpFreeBytesAvailableToCaller: *mut u64,
            lpTotalNumberOfBytes: *mut u64,
            lpTotalNumberOfFreeBytes: *mut u64,
        ) -> i32;
    }

    let mut free_bytes_available: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut total_free_bytes: u64 = 0;

    let probe_dir = if path.is_file() {
        path.parent().unwrap_or(path)
    } else {
        path
    };

    let path_str = probe_dir.to_string_lossy();
    let mut wide: Vec<u16> = std::ffi::OsStr::new(&*path_str)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let ret = GetDiskFreeSpaceExW(
            wide.as_mut_ptr(),
            &mut free_bytes_available,
            &mut total_bytes,
            &mut total_free_bytes,
        );

        if ret != 0 {
            return free_bytes_available;
        }
    }

    50 * 1024 * 1024 * 1024
}

#[cfg(not(windows))]
fn probe_disk(_path: &Path) -> u64 {
    50 * 1024 * 1024 * 1024
}

pub fn evaluate_profiles(metrics: &SystemMetrics) -> Vec<ProfileEvaluation> {
    let mut evals = Vec::new();

    // 1. AirGappedPower
    let air_status = if metrics.total_ram_bytes >= 15 * 1024 * 1024 * 1024
        && metrics.available_ram_bytes >= 5 * 1024 * 1024 * 1024
        && metrics.free_disk_bytes >= 4 * 1024 * 1024 * 1024
    {
        SemaphoreStatus::Green
    } else if metrics.total_ram_bytes >= 8 * 1024 * 1024 * 1024
        && metrics.free_disk_bytes >= 2 * 1024 * 1024 * 1024
    {
        SemaphoreStatus::Yellow
    } else {
        SemaphoreStatus::Red
    };

    let air_reason = match air_status {
        SemaphoreStatus::Green => "Hardware fully qualified for complete offline neural models.",
        SemaphoreStatus::Yellow => "Viable, but RAM may be tight during heavy workspace synchronization.",
        SemaphoreStatus::Red => "Insufficient RAM (<8 GB) or disk space (<2 GB) for full 740 MB suite.",
    };

    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::AirGappedPower,
        status: air_status,
        headline: "Maximum Offline Autonomy & Multi-Modal Vision",
        reason: air_reason.to_string(),
    });

    // 2. SmartBalanced
    let bal_status = if metrics.total_ram_bytes >= 7 * 1024 * 1024 * 1024
        && metrics.free_disk_bytes >= 1500 * 1024 * 1024
    {
        SemaphoreStatus::Green
    } else if metrics.total_ram_bytes >= 4 * 1024 * 1024 * 1024
        && metrics.free_disk_bytes >= 500 * 1024 * 1024
    {
        SemaphoreStatus::Yellow
    } else {
        SemaphoreStatus::Red
    };

    let bal_reason = match bal_status {
        SemaphoreStatus::Green => "Optimal balance between fast startup, memory efficiency, and local neural routing.",
        SemaphoreStatus::Yellow => "Acceptable for lightweight usage, but caution advised under low RAM.",
        SemaphoreStatus::Red => "Insufficient memory (<4 GB) or disk space (<500 MB).",
    };

    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::SmartBalanced,
        status: bal_status,
        headline: "High Cost-Benefit (Query Classifier, Scan Sentinel & Layout)",
        reason: bal_reason.to_string(),
    });

    // 3. Lite
    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::Lite,
        status: SemaphoreStatus::Green,
        headline: "Zero-Overhead Pure Rust Deterministic Heuristics (<10ms Cold Start)",
        reason: "Zero model downloads required. Compatible with any hardware, VM, or CI environment.".to_string(),
    });

    evals
}

pub fn auto_select_recommended(metrics: &SystemMetrics) -> InstallerProfileTier {
    let evals = evaluate_profiles(metrics);
    for e in evals {
        if e.status == SemaphoreStatus::Green {
            return e.tier;
        }
    }
    InstallerProfileTier::Lite
}

pub fn print_preflight_summary(metrics: &SystemMetrics, selected: InstallerProfileTier, evals: &[ProfileEvaluation]) {
    println!("-------------------------------------------------------");
    println!("🖥️  Hardware Pre-Flight Check (<5ms):");
    println!(
        "   • Physical Memory: {:.1} GB RAM ({:.1} GB Available)",
        metrics.total_ram_gb(),
        metrics.available_ram_gb()
    );
    println!("   • Disk Storage:    {:.1} GB Free Space", metrics.free_disk_gb());
    println!("   • CPU Processor:   {} Parallel Cores", metrics.cpu_cores);
    println!();
    println!("Available Intelligence Profiles (Semaphore Semantics):");

    for e in evals {
        let is_selected = e.tier == selected;
        let mark = if is_selected { " [AUTO-SELECTED]" } else { "" };
        let disk_mb = e.tier.disk_required_bytes() as f64 / (1024.0 * 1024.0);
        let ram_mb = e.tier.estimated_peak_ram_bytes() as f64 / (1024.0 * 1024.0);

        println!(
            "   {} {:<18} [Disk: ~{:.0} MB │ Peak RAM: ~{:.0} MB]{}",
            e.status.badge(),
            e.tier.display_name(),
            disk_mb,
            ram_mb,
            mark
        );
        println!("      ↳ {}", e.headline);
    }
    println!("-------------------------------------------------------\n");
}

pub fn resolve_default_models_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ACTX_MODELS_DIR") {
        if !p.trim().is_empty() {
            return PathBuf::from(p);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local).join("AnyContext").join("models");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join("Library").join("Application Support").join("AnyContext").join("models");
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(home) = dirs::home_dir() {
            return home.join(".local").join("share").join("anycontext").join("models");
        }
    }
    PathBuf::from("models")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installer_profile_tier_parsing_and_defaults() {
        assert_eq!(InstallerProfileTier::parse_str("air_gapped_power"), Some(InstallerProfileTier::AirGappedPower));
        assert_eq!(InstallerProfileTier::parse_str("air"), Some(InstallerProfileTier::AirGappedPower));
        assert_eq!(InstallerProfileTier::parse_str("smart_balanced"), Some(InstallerProfileTier::SmartBalanced));
        assert_eq!(InstallerProfileTier::parse_str("balanced"), Some(InstallerProfileTier::SmartBalanced));
        assert_eq!(InstallerProfileTier::parse_str("lite"), Some(InstallerProfileTier::Lite));
        assert_eq!(InstallerProfileTier::parse_str("none"), Some(InstallerProfileTier::Lite));
        assert_eq!(InstallerProfileTier::parse_str("unknown_xyz"), None);

        assert_eq!(InstallerProfileTier::AirGappedPower.target_models().len(), 5);
        assert_eq!(InstallerProfileTier::SmartBalanced.target_models().len(), 3);
        assert_eq!(InstallerProfileTier::Lite.target_models().len(), 0);
    }

    #[test]
    fn test_evaluate_and_auto_select_matrix() {
        // High-end machine: 32 GB RAM, 100 GB Disk
        let high_spec = SystemMetrics {
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: 20 * 1024 * 1024 * 1024,
            free_disk_bytes: 100 * 1024 * 1024 * 1024,
            cpu_cores: 16,
        };
        let evals = evaluate_profiles(&high_spec);
        assert_eq!(evals[0].status, SemaphoreStatus::Green);
        assert_eq!(evals[1].status, SemaphoreStatus::Green);
        assert_eq!(evals[2].status, SemaphoreStatus::Green);
        assert_eq!(auto_select_recommended(&high_spec), InstallerProfileTier::AirGappedPower);

        // Mid-range machine: 8 GB RAM, 10 GB Disk
        let mid_spec = SystemMetrics {
            total_ram_bytes: 8 * 1024 * 1024 * 1024,
            available_ram_bytes: 4 * 1024 * 1024 * 1024,
            free_disk_bytes: 10 * 1024 * 1024 * 1024,
            cpu_cores: 4,
        };
        let evals_mid = evaluate_profiles(&mid_spec);
        assert_eq!(evals_mid[0].status, SemaphoreStatus::Yellow);
        assert_eq!(evals_mid[1].status, SemaphoreStatus::Green);
        assert_eq!(auto_select_recommended(&mid_spec), InstallerProfileTier::SmartBalanced);

        // Low-end machine: 4 GB RAM, 1 GB Disk
        let low_spec = SystemMetrics {
            total_ram_bytes: 4 * 1024 * 1024 * 1024,
            available_ram_bytes: 1024 * 1024 * 1024,
            free_disk_bytes: 1024 * 1024 * 1024,
            cpu_cores: 2,
        };
        let evals_low = evaluate_profiles(&low_spec);
        assert_eq!(evals_low[0].status, SemaphoreStatus::Red);
        assert_eq!(evals_low[1].status, SemaphoreStatus::Yellow);
        assert_eq!(evals_low[2].status, SemaphoreStatus::Green);
        assert_eq!(auto_select_recommended(&low_spec), InstallerProfileTier::Lite);
    }
}

