//! Hardware Acceleration & Execution Device Models for `actx-onnx`.
//!
//! Provides a unified, cross-platform abstraction for neural hardware acceleration:
//! - DirectML on Windows (NVIDIA, AMD, Intel, Qualcomm Snapdragon X Elite)
//! - Metal & CoreML on macOS (Apple Silicon M1-M4 Unified Memory Architecture)
//! - CUDA & Vulkan on Linux / Workstations
//! - CPU SIMD Vectorization fallback

use serde::{Deserialize, Serialize};

/// Hardware vendor for acceleration devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcceleratorVendor {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Qualcomm,
    #[default]
    Unknown,
}

impl AcceleratorVendor {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
            Self::Intel => "Intel",
            Self::Apple => "Apple Silicon",
            Self::Qualcomm => "Qualcomm",
            Self::Unknown => "Generic / Unknown",
        }
    }
}

/// Acceleration backend runtime provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AcceleratorBackend {
    /// Microsoft DirectX 12 Machine Learning (Windows)
    DirectML,
    /// Apple Metal / CoreML Neural Engine (macOS)
    Metal,
    /// NVIDIA CUDA / TensorRT (Linux & Windows)
    Cuda,
    /// Khronos Vulkan Compute (Cross-Platform)
    Vulkan,
    /// Pure CPU SIMD Vectorization (Universal fallback)
    #[default]
    Cpu,
}

impl AcceleratorBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DirectML => "directml",
            Self::Metal => "metal",
            Self::Cuda => "cuda",
            Self::Vulkan => "vulkan",
            Self::Cpu => "cpu",
        }
    }
}

/// Detailed specification of a detected GPU or neural accelerator adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuDevice {
    pub name: String,
    pub vendor: AcceleratorVendor,
    pub backend: AcceleratorBackend,
    pub vram_bytes: u64,
    pub adapter_index: usize,
    pub is_discrete: bool,
}

impl GpuDevice {
    pub fn vram_gb(&self) -> f64 {
        self.vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn badge(&self) -> String {
        match self.backend {
            AcceleratorBackend::DirectML => format!("🚀 DirectML: {}", self.name),
            AcceleratorBackend::Metal => format!("🚀 Metal: {}", self.name),
            AcceleratorBackend::Cuda => format!("🚀 CUDA: {}", self.name),
            AcceleratorBackend::Vulkan => format!("🚀 Vulkan: {}", self.name),
            AcceleratorBackend::Cpu => "💻 CPU SIMD".to_string(),
        }
    }

    pub fn summary(&self) -> String {
        if self.backend == AcceleratorBackend::Cpu {
            "CPU SIMD Multi-Threading (Zero GPU Dependencies)".to_string()
        } else {
            format!(
                "{} ({}) - {:.1} GB Dedicated VRAM [{}]",
                self.name,
                self.vendor.display_name(),
                self.vram_gb(),
                self.backend.as_str().to_uppercase()
            )
        }
    }
}

/// Target hardware acceleration device for local neural model execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionDevice {
    /// Automatic detection: selects the optimal accelerator for the host operating system
    #[default]
    Auto,
    /// Microsoft DirectX 12 Machine Learning (Windows - NVIDIA, AMD, Intel, Qualcomm)
    DirectML { adapter_index: usize },
    /// Apple Metal / CoreML Neural Engine (macOS Apple Silicon M1-M4)
    MetalCoreML,
    /// NVIDIA CUDA / TensorRT (Linux & Windows)
    Cuda { device_id: usize },
    /// Khronos Vulkan Compute (Cross-Platform)
    Vulkan { device_id: usize },
    /// Pure CPU SIMD execution (deterministic, zero GPU dependencies)
    Cpu,
}

impl ExecutionDevice {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::DirectML { .. } => "directml",
            Self::MetalCoreML => "metal_coreml",
            Self::Cuda { .. } => "cuda",
            Self::Vulkan { .. } => "vulkan",
            Self::Cpu => "cpu",
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::Auto => "Auto (System Optimal)".to_string(),
            Self::DirectML { adapter_index } => format!("DirectML GPU (Adapter {})", adapter_index),
            Self::MetalCoreML => "Metal / CoreML (Apple Silicon Unified Memory)".to_string(),
            Self::Cuda { device_id } => format!("NVIDIA CUDA (Device {})", device_id),
            Self::Vulkan { device_id } => format!("Vulkan Compute (Device {})", device_id),
            Self::Cpu => "CPU SIMD Vectorized".to_string(),
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase();
        match clean.as_str() {
            "auto" | "default" => Some(Self::Auto),
            "dml" | "directml" | "dx12" | "gpu" => Some(Self::DirectML { adapter_index: 0 }),
            "metal" | "coreml" | "apple" | "mps" => Some(Self::MetalCoreML),
            "cuda" | "nvidia" => Some(Self::Cuda { device_id: 0 }),
            "vulkan" | "vk" => Some(Self::Vulkan { device_id: 0 }),
            "cpu" | "host" => Some(Self::Cpu),
            _ => None,
        }
    }

    pub fn is_gpu(&self) -> bool {
        !matches!(self, Self::Cpu)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_device_parsing() {
        assert_eq!(ExecutionDevice::parse_str("auto"), Some(ExecutionDevice::Auto));
        assert_eq!(ExecutionDevice::parse_str("dml"), Some(ExecutionDevice::DirectML { adapter_index: 0 }));
        assert_eq!(ExecutionDevice::parse_str("directml"), Some(ExecutionDevice::DirectML { adapter_index: 0 }));
        assert_eq!(ExecutionDevice::parse_str("gpu"), Some(ExecutionDevice::DirectML { adapter_index: 0 }));
        assert_eq!(ExecutionDevice::parse_str("metal"), Some(ExecutionDevice::MetalCoreML));
        assert_eq!(ExecutionDevice::parse_str("coreml"), Some(ExecutionDevice::MetalCoreML));
        assert_eq!(ExecutionDevice::parse_str("cuda"), Some(ExecutionDevice::Cuda { device_id: 0 }));
        assert_eq!(ExecutionDevice::parse_str("vulkan"), Some(ExecutionDevice::Vulkan { device_id: 0 }));
        assert_eq!(ExecutionDevice::parse_str("cpu"), Some(ExecutionDevice::Cpu));
        assert_eq!(ExecutionDevice::parse_str("invalid_xyz"), None);
    }

    #[test]
    fn test_gpu_device_formatting() {
        let gpu = GpuDevice {
            name: "NVIDIA GeForce RTX 4070".to_string(),
            vendor: AcceleratorVendor::Nvidia,
            backend: AcceleratorBackend::DirectML,
            vram_bytes: 12 * 1024 * 1024 * 1024,
            adapter_index: 0,
            is_discrete: true,
        };

        assert_eq!(gpu.badge(), "🚀 DirectML: NVIDIA GeForce RTX 4070");
        assert_eq!(gpu.vram_gb(), 12.0);
        assert!(gpu.summary().contains("12.0 GB"));
    }
}
