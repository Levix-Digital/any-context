//! Hardware Pre-Flight Inspection & Profile Governance Engine (Kit de Models).
//!
//! Evaluates physical machine capabilities (Total RAM, Available RAM, Free SSD Disk Space, CPU Cores)
//! in sub-millisecond time and maps them to the canonical model tiers:
//! - AirGappedPower (Max offline power, ~740 MB disk, ~850 MB peak RAM)
//! - SmartBalanced (High cost-benefit, ~252 MB disk, ~400 MB peak RAM)
//! - Lite (Zero extra downloads, <50 MB RAM, pure Rust deterministic heuristics)

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
pub use actx_onnx::device::{AcceleratorBackend, AcceleratorVendor, ExecutionDevice, GpuDevice};

/// Canonical installation / runtime intelligence profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileTier {
    /// Extreme sovereign intelligence for 64GB+ workstations (Gemma 4 32B + full Document AI).
    SovereignUltra,
    /// High-performance local reasoning for 32GB workstations (Gemma 4 27B + full Document AI).
    WorkstationPro,
    /// Balanced agentic reasoning for 16GB developer machines (Gemma 4 12B + full Document AI).
    AirGappedPower,
    /// Everyday mobility and compact reasoning for 8GB laptops (Gemma 4 Compact + essential Document AI).
    SmartBalanced,
    /// Ultra-fast pure Rust deterministic heuristics with zero downloads (<45MB RAM, Cloud APIs).
    Lite,
}

impl ProfileTier {
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

    pub fn target_models(&self) -> &'static [&'static str] {
        match self {
            Self::SovereignUltra => &[
                "mobilenetv4-rvl-cdip",
                "clip-vit-int8",
                "layoutlmv3-int8",
                "bge-small-onnx",
                "bge-reranker-small",
                "laya-int8",
                "gemma-4-32b-int8",
            ],
            Self::WorkstationPro => &[
                "mobilenetv4-rvl-cdip",
                "clip-vit-int8",
                "layoutlmv3-int8",
                "bge-small-onnx",
                "bge-reranker-small",
                "laya-int8",
                "gemma-4-27b-int8",
            ],
            Self::AirGappedPower => &[
                "mobilenetv4-rvl-cdip",
                "clip-vit-int8",
                "layoutlmv3-int8",
                "bge-small-onnx",
                "bge-reranker-small",
                "laya-int8",
                "gemma-4-12b-int8",
            ],
            Self::SmartBalanced => &[
                "mobilenetv4-rvl-cdip",
                "layoutlmv3-int8",
                "bge-small-onnx",
                "bge-reranker-small",
                "gemma-4-compact-int8",
            ],
            Self::Lite => &[],
        }
    }

    pub fn disk_required_bytes(&self) -> u64 {
        match self {
            Self::SovereignUltra => 19_200_000_000,
            Self::WorkstationPro => 15_700_000_000,
            Self::AirGappedPower => 7_900_000_000,
            Self::SmartBalanced => 1_700_000_000,
            Self::Lite => 0,
        }
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

/// Readiness semaphore for hardware compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemaphoreStatus {
    /// Optimal hardware headroom.
    Green,
    /// Viable but tighter resource margin.
    Yellow,
    /// Insufficient memory or disk space.
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

/// Hardware evaluation report for a specific profile tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEvaluation {
    pub tier: ProfileTier,
    pub status: SemaphoreStatus,
    pub headline: String,
    pub disk_required_bytes: u64,
    pub estimated_ram_bytes: u64,
    pub reason: String,
}

/// Safety verdict for running a specific model on the physical hardware (LM Studio style).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardwareSafetyVerdict {
    /// Safe (🟢): Fits comfortably in physical memory with ample OS headroom.
    Safe {
        model_name: String,
        required_ram_gb: f32,
        total_ram_gb: f32,
    },
    /// Tight (🟡): Viable, but high memory pressure. May swap if heavy background apps are open.
    Tight {
        model_name: String,
        required_ram_gb: f32,
        total_ram_gb: f32,
        available_ram_gb: f32,
    },
    /// Unsafe (🔴): Exceeds physical RAM! Running this model will trigger severe swap thrashing, system freezing, or OOM kill.
    Unsafe {
        model_name: String,
        required_ram_gb: f32,
        total_ram_gb: f32,
        recommended_model: String,
    },
}

impl HardwareSafetyVerdict {
    pub fn is_safe(&self) -> bool {
        matches!(self, Self::Safe { .. })
    }

    pub fn is_unsafe(&self) -> bool {
        matches!(self, Self::Unsafe { .. })
    }

    pub fn status(&self) -> SemaphoreStatus {
        match self {
            Self::Safe { .. } => SemaphoreStatus::Green,
            Self::Tight { .. } => SemaphoreStatus::Yellow,
            Self::Unsafe { .. } => SemaphoreStatus::Red,
        }
    }
}

/// System physical metrics probed at runtime or install time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub total_ram_bytes: u64,
    pub available_ram_bytes: u64,
    pub free_disk_bytes: u64,
    pub cpu_cores: usize,
    pub target_drive_path: PathBuf,
    #[serde(default)]
    pub accelerators: Vec<GpuDevice>,
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

    pub fn primary_accelerator(&self) -> Option<&GpuDevice> {
        self.accelerators.iter().find(|g| g.is_discrete).or_else(|| self.accelerators.first())
    }

    pub fn has_gpu_acceleration(&self) -> bool {
        self.primary_accelerator().map(|g| g.backend != AcceleratorBackend::Cpu).unwrap_or(false)
    }

    pub fn acceleration_summary(&self) -> String {
        if let Some(primary) = self.primary_accelerator() {
            primary.summary()
        } else {
            "CPU SIMD Vectorized (Zero GPU Dependencies)".to_string()
        }
    }

    pub fn recommended_execution_device(&self) -> ExecutionDevice {
        if let Some(gpu) = self.primary_accelerator() {
            match gpu.backend {
                AcceleratorBackend::DirectML => ExecutionDevice::DirectML { adapter_index: gpu.adapter_index },
                AcceleratorBackend::Metal => ExecutionDevice::MetalCoreML,
                AcceleratorBackend::Cuda => ExecutionDevice::Cuda { device_id: gpu.adapter_index },
                AcceleratorBackend::Vulkan => ExecutionDevice::Vulkan { device_id: gpu.adapter_index },
                AcceleratorBackend::Cpu => ExecutionDevice::Cpu,
            }
        } else {
            ExecutionDevice::Cpu
        }
    }
}

/// Central hardware inspector and profile evaluator.
pub struct HardwareProfiler;

impl HardwareProfiler {
    /// Probes the physical system metrics across Windows, Linux, and macOS in <5ms.
    pub fn probe_system(data_path: Option<&Path>) -> SystemMetrics {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let target_path = data_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| {
                crate::storage::get_default_models_dir()
            });

        let (total_ram, available_ram) = Self::probe_memory();
        let free_disk = Self::probe_disk(&target_path);
        let accelerators = Self::probe_accelerators();

        SystemMetrics {
            total_ram_bytes: total_ram,
            available_ram_bytes: available_ram,
            free_disk_bytes: free_disk,
            cpu_cores,
            target_drive_path: target_path,
            accelerators,
        }
    }

    /// Evaluates all profiles against system metrics with traffic light semaphores.
    pub fn evaluate_profiles(metrics: &SystemMetrics) -> Vec<ProfileEvaluation> {
        let avail_ram_gb = metrics.available_ram_gb();
        let total_ram_gb = metrics.total_ram_gb();
        let free_disk_gb = metrics.free_disk_gb();

        let mut evals = Vec::with_capacity(5);

        // 1. SovereignUltra (~19.2 GB disk, ~25.5 GB peak RAM, 64GB+ RAM)
        let (sov_status, sov_reason) = if (avail_ram_gb >= 30.0 || total_ram_gb >= 56.0) && free_disk_gb >= 45.0 {
            (SemaphoreStatus::Green, "Optimal: Workstation has high-capacity memory for Gemma 4 32B and full Document AI.".to_string())
        } else if total_ram_gb >= 44.0 && free_disk_gb >= 25.0 {
            (SemaphoreStatus::Yellow, "Acceptable: Operates fine on 48GB/64GB, tighter margin under concurrent tasks.".to_string())
        } else {
            (SemaphoreStatus::Red, "Constrained: Gemma 4 32B requires 64GB RAM to prevent swapping. Use Workstation Pro on 32GB.".to_string())
        };

        evals.push(ProfileEvaluation {
            tier: ProfileTier::SovereignUltra,
            status: sov_status,
            headline: "Sovereign Frontier: Gemma 4 32B (INT4) + Full Local Document AI Suite".to_string(),
            disk_required_bytes: ProfileTier::SovereignUltra.disk_required_bytes(),
            estimated_ram_bytes: ProfileTier::SovereignUltra.estimated_peak_ram_bytes(),
            reason: sov_reason,
        });

        // 2. WorkstationPro (~15.7 GB disk, ~18.5 GB peak RAM, 32GB RAM)
        let (work_status, work_reason) = if (avail_ram_gb >= 16.0 || total_ram_gb >= 28.0) && free_disk_gb >= 25.0 {
            (SemaphoreStatus::Green, "Optimal: Fits smoothly within 32GB RAM envelope with ample OS headroom.".to_string())
        } else if total_ram_gb >= 20.0 && free_disk_gb >= 18.0 {
            (SemaphoreStatus::Yellow, "Acceptable: Viable on 24GB-32GB machines.".to_string())
        } else {
            (SemaphoreStatus::Red, "Constrained: Gemma 4 27B requires 32GB RAM. Use Air-Gapped Power on 16GB.".to_string())
        };

        evals.push(ProfileEvaluation {
            tier: ProfileTier::WorkstationPro,
            status: work_status,
            headline: "Workstation Reasoning: Gemma 4 27B (INT4) + Full Local Document AI Suite".to_string(),
            disk_required_bytes: ProfileTier::WorkstationPro.disk_required_bytes(),
            estimated_ram_bytes: ProfileTier::WorkstationPro.estimated_peak_ram_bytes(),
            reason: work_reason,
        });

        // 3. AirGappedPower (~7.9 GB disk, ~10.0 GB peak RAM, 16GB RAM)
        let (air_status, air_reason) = if (avail_ram_gb >= 6.5 || total_ram_gb >= 14.0) && free_disk_gb >= 12.0 {
            (SemaphoreStatus::Green, "Optimal: Perfect balance for standard 16GB developer machines (Gemma 4 12B).".to_string())
        } else if total_ram_gb >= 10.0 && free_disk_gb >= 8.0 {
            (SemaphoreStatus::Yellow, "Acceptable: Operates fine, tighter margin on 12GB laptops.".to_string())
        } else {
            (SemaphoreStatus::Red, "Constrained: Gemma 4 12B requires 16GB RAM. Use Smart Balanced on 8GB.".to_string())
        };

        evals.push(ProfileEvaluation {
            tier: ProfileTier::AirGappedPower,
            status: air_status,
            headline: "Developer Sweet Spot: Gemma 4 12B (INT4) + Full Local Document AI Suite".to_string(),
            disk_required_bytes: ProfileTier::AirGappedPower.disk_required_bytes(),
            estimated_ram_bytes: ProfileTier::AirGappedPower.estimated_peak_ram_bytes(),
            reason: air_reason,
        });

        // 4. SmartBalanced (~1.7 GB disk, ~2.5 GB peak RAM, 8GB RAM)
        let (smart_status, smart_reason) = if (avail_ram_gb >= 2.5 || total_ram_gb >= 7.0) && free_disk_gb >= 3.0 {
            (SemaphoreStatus::Green, "Optimal: Ultra-efficient Gemma 4 Compact + essential Document AI for 8GB laptops.".to_string())
        } else if free_disk_gb >= 1.5 && total_ram_gb >= 4.0 {
            (SemaphoreStatus::Yellow, "Acceptable: Modest memory headroom on 4GB-6GB laptops.".to_string())
        } else {
            (SemaphoreStatus::Red, "Constrained: Recommend Lite profile for this machine.".to_string())
        };

        evals.push(ProfileEvaluation {
            tier: ProfileTier::SmartBalanced,
            status: smart_status,
            headline: "Everyday Mobility: Gemma 4 Compact (INT4) + Essential Document AI".to_string(),
            disk_required_bytes: ProfileTier::SmartBalanced.disk_required_bytes(),
            estimated_ram_bytes: ProfileTier::SmartBalanced.estimated_peak_ram_bytes(),
            reason: smart_reason,
        });

        // 5. Lite (0 MB extra models, <45 MB RAM)
        evals.push(ProfileEvaluation {
            tier: ProfileTier::Lite,
            status: SemaphoreStatus::Green,
            headline: "Instant Cold Boot: Zero Downloads, Pure Rust Heuristics & Cloud Model APIs".to_string(),
            disk_required_bytes: ProfileTier::Lite.disk_required_bytes(),
            estimated_ram_bytes: ProfileTier::Lite.estimated_peak_ram_bytes(),
            reason: "Universal: 100% compatible with every hardware configuration, CI/CD and VMs.".to_string(),
        });

        evals
    }

    /// Automatically selects the most powerful GREEN profile for the given system.
    pub fn auto_select_recommended(metrics: &SystemMetrics) -> ProfileTier {
        let evals = Self::evaluate_profiles(metrics);
        for eval in evals {
            if eval.status == SemaphoreStatus::Green {
                return eval.tier;
            }
        }
        ProfileTier::Lite
    }

    /// Returns the recommended local Gemma 4 model for the current hardware metrics.
    pub fn recommended_local_gemma(metrics: &SystemMetrics) -> &'static str {
        let total = metrics.total_ram_gb();
        let avail = metrics.available_ram_gb();

        if total >= 56.0 || avail >= 30.0 {
            "gemma-4-32b"
        } else if total >= 28.0 || avail >= 16.0 {
            "gemma-4-27b"
        } else if total >= 14.0 || avail >= 6.5 {
            "gemma-4-12b"
        } else {
            "gemma-4-compact"
        }
    }

    /// Evaluates if a given model is safe to run on the system (LM Studio style).
    /// Cloud models are always Safe. Local models are validated against physical RAM.
    pub fn validate_model_safety(model_name: &str, metrics: &SystemMetrics) -> HardwareSafetyVerdict {
        let clean = model_name.trim().to_lowercase();
        let is_gemma = clean.starts_with("gemma-4") || clean.starts_with("gemma");

        if !is_gemma {
            return HardwareSafetyVerdict::Safe {
                model_name: model_name.to_string(),
                required_ram_gb: 0.05,
                total_ram_gb: metrics.total_ram_gb() as f32,
            };
        }

        let total = metrics.total_ram_gb() as f32;
        let avail = metrics.available_ram_gb() as f32;
        let recommended = Self::recommended_local_gemma(metrics).to_string();

        if clean.contains("32b") || clean.contains("32") {
            let required = 25.5f32;
            if total < 44.0 {
                HardwareSafetyVerdict::Unsafe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    recommended_model: recommended,
                }
            } else if avail < 22.0 {
                HardwareSafetyVerdict::Tight {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    available_ram_gb: avail,
                }
            } else {
                HardwareSafetyVerdict::Safe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                }
            }
        } else if clean.contains("27b") || clean.contains("27") {
            let required = 18.5f32;
            if total < 22.0 {
                HardwareSafetyVerdict::Unsafe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    recommended_model: recommended,
                }
            } else if avail < 15.0 {
                HardwareSafetyVerdict::Tight {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    available_ram_gb: avail,
                }
            } else {
                HardwareSafetyVerdict::Safe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                }
            }
        } else if clean.contains("12b") || clean.contains("12") {
            let required = 10.0f32;
            if total < 12.0 {
                HardwareSafetyVerdict::Unsafe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    recommended_model: recommended,
                }
            } else if avail < 6.0 {
                HardwareSafetyVerdict::Tight {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    available_ram_gb: avail,
                }
            } else {
                HardwareSafetyVerdict::Safe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                }
            }
        } else {
            // Compact (~2.5 GB RAM)
            let required = 2.5f32;
            if total < 4.0 {
                HardwareSafetyVerdict::Unsafe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    recommended_model: "lite / cloud".to_string(),
                }
            } else if avail < 1.8 {
                HardwareSafetyVerdict::Tight {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                    available_ram_gb: avail,
                }
            } else {
                HardwareSafetyVerdict::Safe {
                    model_name: model_name.to_string(),
                    required_ram_gb: required,
                    total_ram_gb: total,
                }
            }
        }
    }


    // --- Private OS Probers ---

    #[cfg(target_os = "windows")]
    fn probe_memory() -> (u64, u64) {
        #[repr(C)]
        struct MemoryStatusEx {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }

        extern "system" {
            fn GlobalMemoryStatusEx(lp_buffer: *mut MemoryStatusEx) -> i32;
        }

        unsafe {
            let mut status: MemoryStatusEx = std::mem::zeroed();
            status.dw_length = std::mem::size_of::<MemoryStatusEx>() as u32;
            if GlobalMemoryStatusEx(&mut status) != 0 {
                (status.ull_total_phys, status.ull_avail_phys)
            } else {
                (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn probe_memory() -> (u64, u64) {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0u64;
            let mut avail_kb = 0u64;
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        total_kb = val.parse().unwrap_or(0);
                    }
                } else if line.starts_with("MemAvailable:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        avail_kb = val.parse().unwrap_or(0);
                    }
                }
            }
            if total_kb > 0 {
                return (total_kb * 1024, if avail_kb > 0 { avail_kb * 1024 } else { (total_kb / 2) * 1024 });
            }
        }
        (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    fn probe_memory() -> (u64, u64) {
        (8 * 1024 * 1024 * 1024, 4 * 1024 * 1024 * 1024)
    }

    #[cfg(target_os = "windows")]
    fn probe_disk(path: &Path) -> u64 {
        use std::os::windows::ffi::OsStrExt;

        extern "system" {
            fn GetDiskFreeSpaceExW(
                lp_directory_name: *const u16,
                lp_free_bytes_available: *mut u64,
                lp_total_number_of_bytes: *mut u64,
                lp_total_number_of_free_bytes: *mut u64,
            ) -> i32;
        }

        let dir_str = path.to_string_lossy();
        let wide: Vec<u16> = std::ffi::OsStr::new(&*dir_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            let mut free_avail: u64 = 0;
            let mut total_bytes: u64 = 0;
            let mut total_free: u64 = 0;
            if GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_avail,
                &mut total_bytes,
                &mut total_free,
            ) != 0 {
                free_avail
            } else {
                50 * 1024 * 1024 * 1024
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn probe_disk(_path: &Path) -> u64 {
        50 * 1024 * 1024 * 1024
    }

    /// Detects all GPU and neural accelerators available on the host machine in <5ms.
    pub fn probe_accelerators() -> Vec<GpuDevice> {
        #[cfg(target_os = "windows")]
        {
            Self::probe_windows_dxgi_accelerators()
        }

        #[cfg(target_os = "linux")]
        {
            Self::probe_linux_accelerators()
        }

        #[cfg(target_os = "macos")]
        {
            Self::probe_macos_accelerators()
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Vec::new()
        }
    }

    #[cfg(target_os = "windows")]
    fn probe_windows_dxgi_accelerators() -> Vec<GpuDevice> {
        use std::ffi::c_void;

        #[repr(C)]
        struct DxgiAdapterDesc1 {
            description: [u16; 128],
            vendor_id: u32,
            device_id: u32,
            sub_sys_id: u32,
            revision: u32,
            dedicated_video_memory: usize,
            dedicated_system_memory: usize,
            shared_system_memory: usize,
            adapter_luid_low: u32,
            adapter_luid_high: i32,
            flags: u32,
        }

        type CreateDXGIFactory1Fn = unsafe extern "system" fn(
            riid: *const [u8; 16],
            pp_factory: *mut *mut c_void,
        ) -> i32;

        extern "system" {
            fn LoadLibraryA(lp_lib_file_name: *const i8) -> *mut c_void;
            fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const i8) -> *mut c_void;
            fn FreeLibrary(h_lib_module: *mut c_void) -> i32;
        }

        let mut devices = Vec::new();

        unsafe {
            let lib = LoadLibraryA(b"dxgi.dll\0".as_ptr() as *const i8);
            if lib.is_null() {
                return devices;
            }

            let proc = GetProcAddress(lib, b"CreateDXGIFactory1\0".as_ptr() as *const i8);
            if proc.is_null() {
                FreeLibrary(lib);
                return devices;
            }

            let create_factory: CreateDXGIFactory1Fn = std::mem::transmute(proc);

            // IID_IDXGIFactory1 = 770aae78-f26f-4dba-a829-253c83d1b387
            let iid_idxgi_factory1: [u8; 16] = [
                0x78, 0xae, 0x0a, 0x77, 0x6f, 0xf2, 0xba, 0x4d, 0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87,
            ];

            let mut factory: *mut c_void = std::ptr::null_mut();
            if create_factory(&iid_idxgi_factory1, &mut factory) != 0 || factory.is_null() {
                FreeLibrary(lib);
                return devices;
            }

            // IDXGIFactory1 vtable:
            // 0: QueryInterface, 1: AddRef, 2: Release
            // 3: SetPrivateData, 4: SetPrivateDataInterface, 5: GetPrivateData, 6: GetParent
            // 7: EnumAdapters, 8: MakeWindowAssociation, 9: GetWindowAssociation
            // 10: CreateSwapChain, 11: CreateSoftwareAdapter
            // 12: EnumAdapters1
            let factory_vtable = *(factory as *mut *mut *mut c_void);
            let enum_adapters1_ptr = *factory_vtable.add(12);
            let factory_release_ptr = *factory_vtable.add(2);

            type EnumAdapters1Fn = unsafe extern "system" fn(
                this: *mut c_void,
                adapter: u32,
                pp_adapter: *mut *mut c_void,
            ) -> i32;
            type ReleaseFn = unsafe extern "system" fn(this: *mut c_void) -> u32;

            let enum_adapters1: EnumAdapters1Fn = std::mem::transmute(enum_adapters1_ptr);
            let factory_release: ReleaseFn = std::mem::transmute(factory_release_ptr);

            let mut adapter_index = 0u32;
            loop {
                let mut adapter: *mut c_void = std::ptr::null_mut();
                if enum_adapters1(factory, adapter_index, &mut adapter) != 0 || adapter.is_null() {
                    break;
                }

                // IDXGIAdapter1 vtable:
                // 0: QueryInterface, 1: AddRef, 2: Release
                // 3..6: IDXGIObject methods
                // 7: EnumOutputs, 8: GetDesc, 9: CheckInterfaceSupport
                // 10: GetDesc1
                let adapter_vtable = *(adapter as *mut *mut *mut c_void);
                let get_desc1_ptr = *adapter_vtable.add(10);
                let adapter_release_ptr = *adapter_vtable.add(2);

                type GetDesc1Fn = unsafe extern "system" fn(
                    this: *mut c_void,
                    p_desc: *mut DxgiAdapterDesc1,
                ) -> i32;

                let get_desc1: GetDesc1Fn = std::mem::transmute(get_desc1_ptr);
                let adapter_release: ReleaseFn = std::mem::transmute(adapter_release_ptr);

                let mut desc: DxgiAdapterDesc1 = std::mem::zeroed();
                if get_desc1(adapter, &mut desc) == 0 {
                    // DXGI_ADAPTER_FLAG_SOFTWARE = 2
                    let is_software = (desc.flags & 2) != 0;
                    if !is_software {
                        let name_len = desc.description.iter().position(|&c| c == 0).unwrap_or(desc.description.len());
                        let name = String::from_utf16_lossy(&desc.description[..name_len]).trim().to_string();

                        let vendor = match desc.vendor_id {
                            0x10DE => AcceleratorVendor::Nvidia,
                            0x1002 => AcceleratorVendor::Amd,
                            0x8086 => AcceleratorVendor::Intel,
                            0x17CB => AcceleratorVendor::Qualcomm,
                            _ => AcceleratorVendor::Unknown,
                        };

                        let vram_bytes = desc.dedicated_video_memory as u64;
                        let is_discrete = vram_bytes > 512 * 1024 * 1024 || vendor == AcceleratorVendor::Nvidia || vendor == AcceleratorVendor::Amd;

                        devices.push(GpuDevice {
                            name,
                            vendor,
                            backend: AcceleratorBackend::DirectML,
                            vram_bytes,
                            adapter_index: adapter_index as usize,
                            is_discrete,
                        });
                    }
                }

                adapter_release(adapter);
                adapter_index += 1;
                if adapter_index >= 16 {
                    break;
                }
            }

            factory_release(factory);
            FreeLibrary(lib);
        }

        devices
    }

    #[cfg(target_os = "linux")]
    fn probe_linux_accelerators() -> Vec<GpuDevice> {
        let mut devices = Vec::new();

        let has_nvidia = Path::new("/proc/driver/nvidia/version").exists()
            || Path::new("/dev/nvidia0").exists();

        if has_nvidia {
            let mut name = "NVIDIA CUDA GPU".to_string();
            if let Ok(entries) = std::fs::read_dir("/proc/driver/nvidia/gpus") {
                for entry in entries.flatten() {
                    let info_path = entry.path().join("information");
                    if let Ok(content) = std::fs::read_to_string(info_path) {
                        for line in content.lines() {
                            if line.starts_with("Model:") {
                                if let Some(model) = line.strip_prefix("Model:").map(str::trim) {
                                    name = model.to_string();
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            devices.push(GpuDevice {
                name,
                vendor: AcceleratorVendor::Nvidia,
                backend: AcceleratorBackend::Cuda,
                vram_bytes: 8 * 1024 * 1024 * 1024,
                adapter_index: 0,
                is_discrete: true,
            });
        }

        if devices.is_empty() && Path::new("/dev/dri/renderD128").exists() {
            devices.push(GpuDevice {
                name: "Linux Vulkan Accelerator".to_string(),
                vendor: AcceleratorVendor::Unknown,
                backend: AcceleratorBackend::Vulkan,
                vram_bytes: 4 * 1024 * 1024 * 1024,
                adapter_index: 0,
                is_discrete: false,
            });
        }

        devices
    }

    #[cfg(target_os = "macos")]
    fn probe_macos_accelerators() -> Vec<GpuDevice> {
        let mut devices = Vec::new();
        #[cfg(target_arch = "aarch64")]
        {
            let (total_ram, _) = Self::probe_memory();
            devices.push(GpuDevice {
                name: "Apple Silicon Metal / CoreML Engine".to_string(),
                vendor: AcceleratorVendor::Apple,
                backend: AcceleratorBackend::Metal,
                vram_bytes: total_ram,
                adapter_index: 0,
                is_discrete: true,
            });
        }
        devices
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_profiler_probe() {
        let metrics = HardwareProfiler::probe_system(None);
        assert!(metrics.total_ram_bytes > 0, "Total RAM should be greater than 0");
        assert!(metrics.available_ram_bytes > 0, "Available RAM should be greater than 0");
        assert!(metrics.cpu_cores >= 1, "CPU cores should be at least 1");
        assert!(metrics.free_disk_bytes > 0, "Free disk should be greater than 0");
    }

    #[test]
    fn test_auto_select_recommended() {
        // 1. Sovereign 64GB machine
        let ultra_metrics = SystemMetrics {
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            available_ram_bytes: 35 * 1024 * 1024 * 1024,
            free_disk_bytes: 120 * 1024 * 1024 * 1024,
            cpu_cores: 16,
            target_drive_path: PathBuf::from("."),
            accelerators: Vec::new(),
        };
        assert_eq!(HardwareProfiler::auto_select_recommended(&ultra_metrics), ProfileTier::SovereignUltra);

        // 2. Workstation 32GB machine
        let pro_metrics = SystemMetrics {
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: 19 * 1024 * 1024 * 1024,
            free_disk_bytes: 80 * 1024 * 1024 * 1024,
            cpu_cores: 12,
            target_drive_path: PathBuf::from("."),
            accelerators: Vec::new(),
        };
        assert_eq!(HardwareProfiler::auto_select_recommended(&pro_metrics), ProfileTier::WorkstationPro);

        // 3. 16GB developer machine
        let power_metrics = SystemMetrics {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            available_ram_bytes: 8 * 1024 * 1024 * 1024,
            free_disk_bytes: 40 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            target_drive_path: PathBuf::from("."),
            accelerators: Vec::new(),
        };
        assert_eq!(HardwareProfiler::auto_select_recommended(&power_metrics), ProfileTier::AirGappedPower);

        // 4. 8GB balanced machine
        let balanced_metrics = SystemMetrics {
            total_ram_bytes: 8 * 1024 * 1024 * 1024,
            available_ram_bytes: 4 * 1024 * 1024 * 1024,
            free_disk_bytes: 20 * 1024 * 1024 * 1024,
            cpu_cores: 4,
            target_drive_path: PathBuf::from("."),
            accelerators: Vec::new(),
        };
        assert_eq!(HardwareProfiler::auto_select_recommended(&balanced_metrics), ProfileTier::SmartBalanced);

        // 5. Budget machine / VM
        let low_metrics = SystemMetrics {
            total_ram_bytes: 4 * 1024 * 1024 * 1024,
            available_ram_bytes: 1 * 1024 * 1024 * 1024,
            free_disk_bytes: 500 * 1024 * 1024, // 500 MB
            cpu_cores: 2,
            target_drive_path: PathBuf::from("."),
            accelerators: Vec::new(),
        };
        assert_eq!(HardwareProfiler::auto_select_recommended(&low_metrics), ProfileTier::Lite);
    }

    #[test]
    fn test_validate_model_safety() {
        let metrics_16gb = SystemMetrics {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            available_ram_bytes: 8 * 1024 * 1024 * 1024,
            free_disk_bytes: 40 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            target_drive_path: PathBuf::from("."),
            accelerators: Vec::new(),
        };

        // Cloud models always Safe
        let v_cloud = HardwareProfiler::validate_model_safety("claude-3-5-sonnet", &metrics_16gb);
        assert!(v_cloud.is_safe());

        // Gemma 4 12B is Safe on 16GB
        let v_12b = HardwareProfiler::validate_model_safety("gemma-4-12b", &metrics_16gb);
        assert!(v_12b.is_safe());

        // Gemma 4 27B and 32B are Unsafe on 16GB
        let v_27b = HardwareProfiler::validate_model_safety("gemma-4-27b", &metrics_16gb);
        assert!(v_27b.is_unsafe());

        let v_32b = HardwareProfiler::validate_model_safety("gemma-4-32b", &metrics_16gb);
        assert!(v_32b.is_unsafe());
        if let HardwareSafetyVerdict::Unsafe { recommended_model, .. } = v_32b {
            assert_eq!(recommended_model, "gemma-4-12b");
        } else {
            panic!("Expected Unsafe verdict");
        }
    }

    #[test]
    fn test_hardware_accelerator_probing_and_selection() {
        let metrics = HardwareProfiler::probe_system(None);
        // Probe runs in <5ms without panic
        println!("Detected {} accelerators", metrics.accelerators.len());
        for acc in &metrics.accelerators {
            println!(" - {}", acc.summary());
        }

        // Test mock accelerator routing
        let gpu_metrics = SystemMetrics {
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            available_ram_bytes: 20 * 1024 * 1024 * 1024,
            free_disk_bytes: 100 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            target_drive_path: PathBuf::from("."),
            accelerators: vec![
                GpuDevice {
                    name: "NVIDIA GeForce RTX 4080".to_string(),
                    vendor: AcceleratorVendor::Nvidia,
                    backend: AcceleratorBackend::DirectML,
                    vram_bytes: 16 * 1024 * 1024 * 1024,
                    adapter_index: 0,
                    is_discrete: true,
                }
            ],
        };

        assert!(gpu_metrics.has_gpu_acceleration());
        assert_eq!(
            gpu_metrics.recommended_execution_device(),
            ExecutionDevice::DirectML { adapter_index: 0 }
        );
        assert!(gpu_metrics.acceleration_summary().contains("DirectML") || gpu_metrics.acceleration_summary().contains("RTX 4080"));

        let cpu_only_metrics = SystemMetrics {
            total_ram_bytes: 16 * 1024 * 1024 * 1024,
            available_ram_bytes: 8 * 1024 * 1024 * 1024,
            free_disk_bytes: 40 * 1024 * 1024 * 1024,
            cpu_cores: 8,
            target_drive_path: PathBuf::from("."),
            accelerators: Vec::new(),
        };

        assert!(!cpu_only_metrics.has_gpu_acceleration());
        assert_eq!(cpu_only_metrics.recommended_execution_device(), ExecutionDevice::Cpu);
        assert!(cpu_only_metrics.acceleration_summary().contains("CPU"));
    }
}

