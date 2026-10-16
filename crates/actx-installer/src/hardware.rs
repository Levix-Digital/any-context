//! Sub-millisecond hardware pre-flight inspection and model profile selector for actx-installer.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallerProfileTier {
    SovereignUltra,
    WorkstationPro,
    AirGappedPower,
    SmartBalanced,
    Lite,
}

impl InstallerProfileTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SovereignUltra => "sovereign_ultra",
            Self::WorkstationPro => "workstation_pro",
            Self::AirGappedPower => "air_gapped_power",
            Self::SmartBalanced => "smart_balanced",
            Self::Lite => "lite",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SovereignUltra => "Sovereign Ultra (64GB+)",
            Self::WorkstationPro => "Workstation Pro (32GB)",
            Self::AirGappedPower => "Air-Gapped Power (16GB)",
            Self::SmartBalanced => "Smart Balanced (8GB)",
            Self::Lite => "Lite / Cloud (<8GB)",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase();
        match clean.as_str() {
            "sovereign_ultra" | "sovereign" | "ultra" | "64gb" | "sovereign-ultra" => Some(Self::SovereignUltra),
            "workstation_pro" | "workstation" | "pro" | "32gb" | "workstation-pro" => Some(Self::WorkstationPro),
            "air_gapped_power" | "airgapped" | "air-gapped" | "air" | "max" | "power" | "16gb" => Some(Self::AirGappedPower),
            "smart_balanced" | "balanced" | "smart" | "medium" | "8gb" | "smart-balanced" => Some(Self::SmartBalanced),
            "lite" | "fast" | "fast-path" | "minimal" | "none" | "cloud" => Some(Self::Lite),
            _ => None,
        }
    }

    pub fn target_models(&self) -> &'static [ModelDownloadSpec] {
        match self {
            Self::SovereignUltra => &SOVEREIGN_MODELS,
            Self::WorkstationPro => &WORKSTATION_MODELS,
            Self::AirGappedPower => &POWER_MODELS,
            Self::SmartBalanced => &BALANCED_MODELS,
            Self::Lite => &[],
        }
    }

    /// Disk needed by the weights this profile installs (Gemma 4 is planned and not downloaded).
    pub fn disk_required_bytes(&self) -> u64 {
        self.target_models().iter().map(|m| m.size_bytes).sum()
    }

    pub fn estimated_peak_ram_bytes(&self) -> u64 {
        match self {
            Self::SovereignUltra => 25_500_000_000,
            Self::WorkstationPro => 18_500_000_000,
            Self::AirGappedPower => 10_000_000_000,
            Self::SmartBalanced => 2_500_000_000,
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

/// A verified downloadable model file (mirrors `any-context-core-rs` `ONNX_CATALOG`, ADR-130).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelDownloadSpec {
    pub id: &'static str,
    /// Path relative to the models directory (`/`-separated).
    pub file_name: &'static str,
    pub size_bytes: u64,
    /// Upstream URL pinned to an immutable revision.
    pub download_url: &'static str,
    /// Lowercase hex SHA-256 of the file.
    pub sha256: &'static str,
    pub description: &'static str,
}

/// Document AI weights (their components currently run deterministic heuristics).
pub static ALL_MODELS: [ModelDownloadSpec; 5] = [
    ModelDownloadSpec {
        id: "mobilenetv4-rvl-cdip",
        file_name: "mobilenetv4_rvl_cdip.onnx",
        size_bytes: 3_931_745,
        download_url: "https://huggingface.co/onnx-community/mobilenetv4_conv_small.e1200_r224_in1k/resolve/e832d3b2b2fd988f41063559c6ea30df281f4e05/onnx/model_int8.onnx",
        sha256: "9930547d7d61660d667b72069f0fa75b706b3e0b04645cf99f76416a82fa99a3",
        description: "Scanned-page sentinel weights",
    },
    ModelDownloadSpec {
        id: "layoutlmv3-int8",
        file_name: "layoutlmv3_base_int8.onnx",
        size_bytes: 127_001_085,
        download_url: "https://huggingface.co/onnx-community/layoutlmv3-base-ONNX/resolve/43d4a6517dc9b9a5eb3d538a158161ead8d028b9/onnx/model_int8.onnx",
        sha256: "5f75612d2c2cd3b23826b7fcbed33e11f39d6acda1fe66fa091a0392b18a9013",
        description: "Layout extraction weights",
    },
    ModelDownloadSpec {
        id: "bge-small-onnx",
        file_name: "bge_small_query_int8.onnx",
        size_bytes: 133_093_490,
        download_url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a/onnx/model.onnx",
        sha256: "828e1496d7fabb79cfa4dcd84fa38625c0d3d21da474a00f08db0f559940cf35",
        description: "Query routing weights",
    },
    ModelDownloadSpec {
        id: "clip-vit-int8",
        file_name: "clip_vit_vision_int8.onnx",
        size_bytes: 88_648_877,
        download_url: "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/d15189d7028b43f1d3e65039190477f6af591c2a/onnx/vision_model_int8.onnx",
        sha256: "0ab0c1b3ace708e539633af1744d5a95247fe4e14d3e08ff197ef82a6cb9bd93",
        description: "Visual encoder weights",
    },
    ModelDownloadSpec {
        id: "laya-int8",
        file_name: "laya_mmbert_int8.onnx",
        size_bytes: 424_348_081,
        download_url: "https://huggingface.co/tozp/laya-onnx/resolve/0d1f7ebf46a3ea04ec4424df602f96ddefb66766/model_int8.onnx",
        sha256: "d337ce1b1cbca907a4063223517af6db7e89f5c9e8d6a2f6a289babc256f4469",
        description: "Document typology weights",
    },
];

/// Default multilingual cross-encoder (model + tokenizer), provisioned automatically
/// with every non-Lite profile so neural re-ranking works without user action (D15).
pub static RERANKER_FILES: [ModelDownloadSpec; 2] = [
    ModelDownloadSpec {
        id: "mmarco-reranker",
        file_name: "mmarco-reranker/model.onnx",
        size_bytes: 118_620_016,
        download_url: "https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1/resolve/1427fd652930e4ba29e8149678df786c240d8825/onnx/model_quint8_avx2.onnx",
        sha256: "6c2513767fb63d008a4377bef7a7a3555433d9436342bb53e35a3a72ffc52d4b",
        description: "Multilingual cross-encoder re-ranker (model)",
    },
    ModelDownloadSpec {
        id: "mmarco-reranker",
        file_name: "mmarco-reranker/tokenizer.json",
        size_bytes: 17_082_660,
        download_url: "https://huggingface.co/cross-encoder/mmarco-mMiniLMv2-L12-H384-v1/resolve/1427fd652930e4ba29e8149678df786c240d8825/tokenizer.json",
        sha256: "62c24cdc13d4c9952d63718d6c9fa4c287974249e16b7ade6d5a85e7bbb75626",
        description: "Multilingual cross-encoder re-ranker (tokenizer)",
    },
];

// Gemma 4 local generation is planned (ADR-130): no profile downloads Gemma weights.

pub static SOVEREIGN_MODELS: [ModelDownloadSpec; 7] = [
    ALL_MODELS[0], ALL_MODELS[1], ALL_MODELS[2], ALL_MODELS[3], ALL_MODELS[4],
    RERANKER_FILES[0], RERANKER_FILES[1],
];

pub static WORKSTATION_MODELS: [ModelDownloadSpec; 7] = [
    ALL_MODELS[0], ALL_MODELS[1], ALL_MODELS[2], ALL_MODELS[3], ALL_MODELS[4],
    RERANKER_FILES[0], RERANKER_FILES[1],
];

pub static POWER_MODELS: [ModelDownloadSpec; 7] = [
    ALL_MODELS[0], ALL_MODELS[1], ALL_MODELS[2], ALL_MODELS[3], ALL_MODELS[4],
    RERANKER_FILES[0], RERANKER_FILES[1],
];

pub static BALANCED_MODELS: [ModelDownloadSpec; 5] = [
    ALL_MODELS[0], ALL_MODELS[1], ALL_MODELS[2],
    RERANKER_FILES[0], RERANKER_FILES[1],
];

impl InstallerProfileTier {
    /// Files installed automatically for this profile. Only components that execute real
    /// inference are provisioned (today: the cross-encoder re-ranker); Lite installs nothing.
    pub fn auto_provisioned_models(&self) -> &'static [ModelDownloadSpec] {
        match self {
            Self::Lite => &[],
            _ => &RERANKER_FILES,
        }
    }
}

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

/// Free-disk headroom (beyond the profile's weights) for a Green / Yellow rating.
const GREEN_DISK_HEADROOM_GB: f64 = 1.0;
const YELLOW_DISK_HEADROOM_GB: f64 = 0.25;

fn disk_fits(free_disk_gb: f64, tier: InstallerProfileTier, headroom_gb: f64) -> bool {
    free_disk_gb >= tier.disk_required_bytes() as f64 / (1024.0 * 1024.0 * 1024.0) + headroom_gb
}

pub fn evaluate_profiles(metrics: &SystemMetrics) -> Vec<ProfileEvaluation> {
    let avail_ram_gb = metrics.available_ram_gb();
    let total_ram_gb = metrics.total_ram_gb();
    let free_disk_gb = metrics.free_disk_gb();

    let mut evals = Vec::with_capacity(5);

    // 1. SovereignUltra (~25.5 GB peak RAM, 64GB+ RAM)
    let (sov_status, sov_reason) = if (avail_ram_gb >= 30.0 || total_ram_gb >= 56.0) && disk_fits(free_disk_gb, InstallerProfileTier::SovereignUltra, GREEN_DISK_HEADROOM_GB) {
        (SemaphoreStatus::Green, "Optimal: Workstation has high-capacity memory for Gemma 4 32B and full Document AI.")
    } else if total_ram_gb >= 44.0 && disk_fits(free_disk_gb, InstallerProfileTier::SovereignUltra, YELLOW_DISK_HEADROOM_GB) {
        (SemaphoreStatus::Yellow, "Acceptable: Operates fine on 48GB/64GB, tighter margin under concurrent tasks.")
    } else {
        (SemaphoreStatus::Red, "Constrained: Gemma 4 32B requires 64GB RAM to prevent swapping. Use Workstation Pro on 32GB.")
    };

    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::SovereignUltra,
        status: sov_status,
        headline: "Sovereign Frontier: Gemma 4 32B (planned) + Full Local Document AI Suite",
        reason: sov_reason.to_string(),
    });

    // 2. WorkstationPro (~18.5 GB peak RAM, 32GB RAM)
    let (work_status, work_reason) = if (avail_ram_gb >= 16.0 || total_ram_gb >= 28.0) && disk_fits(free_disk_gb, InstallerProfileTier::WorkstationPro, GREEN_DISK_HEADROOM_GB) {
        (SemaphoreStatus::Green, "Optimal: Fits smoothly within 32GB RAM envelope with ample OS headroom.")
    } else if total_ram_gb >= 20.0 && disk_fits(free_disk_gb, InstallerProfileTier::WorkstationPro, YELLOW_DISK_HEADROOM_GB) {
        (SemaphoreStatus::Yellow, "Acceptable: Viable on 24GB-32GB machines.")
    } else {
        (SemaphoreStatus::Red, "Constrained: Gemma 4 27B requires 32GB RAM. Use Air-Gapped Power on 16GB.")
    };

    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::WorkstationPro,
        status: work_status,
        headline: "Workstation Reasoning: Gemma 4 27B (planned) + Full Local Document AI Suite",
        reason: work_reason.to_string(),
    });

    // 3. AirGappedPower (~10.0 GB peak RAM, 16GB RAM)
    let (air_status, air_reason) = if (avail_ram_gb >= 6.5 || total_ram_gb >= 14.0) && disk_fits(free_disk_gb, InstallerProfileTier::AirGappedPower, GREEN_DISK_HEADROOM_GB) {
        (SemaphoreStatus::Green, "Optimal: Perfect balance for standard 16GB developer machines (Gemma 4 12B).")
    } else if total_ram_gb >= 10.0 && disk_fits(free_disk_gb, InstallerProfileTier::AirGappedPower, YELLOW_DISK_HEADROOM_GB) {
        (SemaphoreStatus::Yellow, "Acceptable: Operates fine, tighter margin on 12GB laptops.")
    } else {
        (SemaphoreStatus::Red, "Constrained: Gemma 4 12B requires 16GB RAM. Use Smart Balanced on 8GB.")
    };

    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::AirGappedPower,
        status: air_status,
        headline: "Developer Sweet Spot: Gemma 4 12B (planned) + Full Local Document AI Suite",
        reason: air_reason.to_string(),
    });

    // 4. SmartBalanced (~2.5 GB peak RAM, 8GB RAM)
    let (bal_status, bal_reason) = if (avail_ram_gb >= 2.5 || total_ram_gb >= 7.0) && disk_fits(free_disk_gb, InstallerProfileTier::SmartBalanced, GREEN_DISK_HEADROOM_GB) {
        (SemaphoreStatus::Green, "Optimal: Ultra-efficient Gemma 4 Compact + essential Document AI for 8GB laptops.")
    } else if disk_fits(free_disk_gb, InstallerProfileTier::SmartBalanced, YELLOW_DISK_HEADROOM_GB) && total_ram_gb >= 4.0 {
        (SemaphoreStatus::Yellow, "Acceptable: Modest memory headroom on 4GB-6GB laptops.")
    } else {
        (SemaphoreStatus::Red, "Constrained: Recommend Lite profile for this machine.")
    };

    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::SmartBalanced,
        status: bal_status,
        headline: "Everyday Mobility: Gemma 4 Compact (planned) + Essential Document AI",
        reason: bal_reason.to_string(),
    });

    // 5. Lite
    evals.push(ProfileEvaluation {
        tier: InstallerProfileTier::Lite,
        status: SemaphoreStatus::Green,
        headline: "Instant Cold Boot: Zero Downloads, Pure Rust Heuristics & Cloud Model APIs",
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
        let disk_str = if e.tier.disk_required_bytes() >= 1_000_000_000 {
            format!("{:.1} GB", e.tier.disk_required_bytes() as f64 / (1024.0 * 1024.0 * 1024.0))
        } else {
            format!("{:.0} MB", e.tier.disk_required_bytes() as f64 / (1024.0 * 1024.0))
        };

        let ram_str = if e.tier.estimated_peak_ram_bytes() >= 1_000_000_000 {
            format!("{:.1} GB", e.tier.estimated_peak_ram_bytes() as f64 / (1024.0 * 1024.0 * 1024.0))
        } else {
            format!("{:.0} MB", e.tier.estimated_peak_ram_bytes() as f64 / (1024.0 * 1024.0))
        };

        println!(
            "   {} {:<28} [Disk: ~{} │ Peak RAM: ~{}]{}",
            e.status.badge(),
            e.tier.display_name(),
            disk_str,
            ram_str,
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
        assert_eq!(InstallerProfileTier::parse_str("sovereign_ultra"), Some(InstallerProfileTier::SovereignUltra));
        assert_eq!(InstallerProfileTier::parse_str("sovereign"), Some(InstallerProfileTier::SovereignUltra));
        assert_eq!(InstallerProfileTier::parse_str("workstation_pro"), Some(InstallerProfileTier::WorkstationPro));
        assert_eq!(InstallerProfileTier::parse_str("workstation"), Some(InstallerProfileTier::WorkstationPro));
        assert_eq!(InstallerProfileTier::parse_str("air_gapped_power"), Some(InstallerProfileTier::AirGappedPower));
        assert_eq!(InstallerProfileTier::parse_str("air"), Some(InstallerProfileTier::AirGappedPower));
        assert_eq!(InstallerProfileTier::parse_str("smart_balanced"), Some(InstallerProfileTier::SmartBalanced));
        assert_eq!(InstallerProfileTier::parse_str("balanced"), Some(InstallerProfileTier::SmartBalanced));
        assert_eq!(InstallerProfileTier::parse_str("lite"), Some(InstallerProfileTier::Lite));
        assert_eq!(InstallerProfileTier::parse_str("none"), Some(InstallerProfileTier::Lite));
        assert_eq!(InstallerProfileTier::parse_str("unknown_xyz"), None);

        assert_eq!(InstallerProfileTier::SovereignUltra.target_models().len(), 7);
        assert_eq!(InstallerProfileTier::WorkstationPro.target_models().len(), 7);
        assert_eq!(InstallerProfileTier::AirGappedPower.target_models().len(), 7);
        assert_eq!(InstallerProfileTier::SmartBalanced.target_models().len(), 5);
        assert_eq!(InstallerProfileTier::Lite.target_models().len(), 0);
    }

    #[test]
    fn test_evaluate_and_auto_select_matrix() {
        // Ultra machine: 64 GB RAM, 100 GB Disk
        let ultra_spec = SystemMetrics {
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            available_ram_bytes: 40 * 1024 * 1024 * 1024,
            free_disk_bytes: 100 * 1024 * 1024 * 1024,
            cpu_cores: 32,
        };
        let evals_ultra = evaluate_profiles(&ultra_spec);
        assert_eq!(evals_ultra[0].status, SemaphoreStatus::Green);
        assert_eq!(auto_select_recommended(&ultra_spec), InstallerProfileTier::SovereignUltra);

        // High-end workstation: 32 GB RAM, 50 GB Disk
        let high_spec = SystemMetrics {
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: 20 * 1024 * 1024 * 1024,
            free_disk_bytes: 50 * 1024 * 1024 * 1024,
            cpu_cores: 16,
        };
        let evals = evaluate_profiles(&high_spec);
        assert_eq!(evals[0].status, SemaphoreStatus::Red); // Sovereign requires 64GB
        assert_eq!(evals[1].status, SemaphoreStatus::Green); // WorkstationPro
        assert_eq!(evals[2].status, SemaphoreStatus::Green); // AirGappedPower
        assert_eq!(evals[3].status, SemaphoreStatus::Green); // SmartBalanced
        assert_eq!(evals[4].status, SemaphoreStatus::Green); // Lite
        assert_eq!(auto_select_recommended(&high_spec), InstallerProfileTier::WorkstationPro);

        // Standard developer laptop: 16 GB RAM, 20 GB Disk
        let dev_spec = SystemMetrics {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            available_ram_bytes: 10 * 1024 * 1024 * 1024,
            free_disk_bytes: 20 * 1024 * 1024 * 1024,
            cpu_cores: 8,
        };
        let evals_dev = evaluate_profiles(&dev_spec);
        assert_eq!(evals_dev[0].status, SemaphoreStatus::Red);
        assert_eq!(evals_dev[1].status, SemaphoreStatus::Red);
        assert_eq!(evals_dev[2].status, SemaphoreStatus::Green); // AirGappedPower
        assert_eq!(auto_select_recommended(&dev_spec), InstallerProfileTier::AirGappedPower);

        // Mid-range machine: 8 GB RAM, 10 GB Disk
        let mid_spec = SystemMetrics {
            total_ram_bytes: 8 * 1024 * 1024 * 1024,
            available_ram_bytes: 4 * 1024 * 1024 * 1024,
            free_disk_bytes: 10 * 1024 * 1024 * 1024,
            cpu_cores: 4,
        };
        let evals_mid = evaluate_profiles(&mid_spec);
        assert_eq!(evals_mid[0].status, SemaphoreStatus::Red);
        assert_eq!(evals_mid[1].status, SemaphoreStatus::Red);
        assert_eq!(evals_mid[2].status, SemaphoreStatus::Red);
        assert_eq!(evals_mid[3].status, SemaphoreStatus::Green); // SmartBalanced
        assert_eq!(auto_select_recommended(&mid_spec), InstallerProfileTier::SmartBalanced);

        // Low-end machine: 4 GB RAM, 512 MB Disk
        let low_spec = SystemMetrics {
            total_ram_bytes: 4 * 1024 * 1024 * 1024,
            available_ram_bytes: 1024 * 1024 * 1024,
            free_disk_bytes: 512 * 1024 * 1024,
            cpu_cores: 2,
        };
        let evals_low = evaluate_profiles(&low_spec);
        assert_eq!(evals_low[0].status, SemaphoreStatus::Red);
        assert_eq!(evals_low[1].status, SemaphoreStatus::Red);
        assert_eq!(evals_low[2].status, SemaphoreStatus::Red);
        assert_eq!(evals_low[3].status, SemaphoreStatus::Red);
        assert_eq!(evals_low[4].status, SemaphoreStatus::Green);
        assert_eq!(auto_select_recommended(&low_spec), InstallerProfileTier::Lite);
    }

    #[test]
    fn test_disk_requirements_match_installed_weights() {
        // Planned Gemma weights are never downloaded, so no profile needs gigabytes of disk.
        for tier in [
            InstallerProfileTier::SovereignUltra,
            InstallerProfileTier::WorkstationPro,
            InstallerProfileTier::AirGappedPower,
            InstallerProfileTier::SmartBalanced,
        ] {
            let expected: u64 = tier.target_models().iter().map(|m| m.size_bytes).sum();
            assert_eq!(tier.disk_required_bytes(), expected);
            assert!(tier.disk_required_bytes() < 1_000_000_000);
        }
        assert_eq!(InstallerProfileTier::Lite.disk_required_bytes(), 0);

        // 16 GB RAM with only 2 GB free disk still fits a model-backed profile.
        let tight_disk = SystemMetrics {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            available_ram_bytes: 8 * 1024 * 1024 * 1024,
            free_disk_bytes: 2 * 1024 * 1024 * 1024,
            cpu_cores: 8,
        };
        assert_ne!(auto_select_recommended(&tight_disk), InstallerProfileTier::Lite);
    }
}

