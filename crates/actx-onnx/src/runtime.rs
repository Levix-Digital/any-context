//! Real ONNX Runtime inference sessions (ADR-130).
//!
//! Wraps `ort` (ONNX Runtime bindings) behind the `runtime` Cargo feature.
//! Sessions run on the CPU Execution Provider only; GPU Execution Providers
//! are planned and intentionally not instantiated in this build.
//!
//! The ONNX Runtime shared library is loaded dynamically and lazily, from
//! `ORT_DYLIB_PATH` when set, otherwise from the executable's directory, where the
//! release package ships it. If it is missing or incompatible, every load fails with
//! a descriptive error and callers fall back to their deterministic paths (ADR-110).
//!
//! Without the `runtime` feature every load fails the same way.

use std::path::{Path, PathBuf};

/// File name of the ONNX Runtime shared library for this platform.
pub fn runtime_library_file_name() -> String {
    format!("{}onnxruntime{}", std::env::consts::DLL_PREFIX, std::env::consts::DLL_SUFFIX)
}

/// True when this build links the runtime feature and the shared library file exists.
/// Cheap filesystem check; it does not load the library.
pub fn runtime_library_present() -> bool {
    runtime_available() && runtime_library_path().map(|p| p.is_file()).unwrap_or(false)
}

/// Resolves where the ONNX Runtime shared library is expected:
/// `ORT_DYLIB_PATH` if set, otherwise next to the running executable.
pub fn runtime_library_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ORT_DYLIB_PATH") {
        if !p.trim().is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(runtime_library_file_name())))
}

/// Flattened `f32` output tensor returned by an inference run.
#[derive(Debug, Clone, PartialEq)]
pub struct TensorOutput {
    pub shape: Vec<i64>,
    pub data: Vec<f32>,
}

/// Returns true when this build links a real ONNX Runtime.
pub const fn runtime_available() -> bool {
    cfg!(feature = "runtime")
}

/// Number of intra-op threads used by CPU sessions (physical parallelism capped at 4).
pub fn default_intra_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .clamp(1, 4)
}

#[cfg(feature = "runtime")]
mod imp {
    use super::TensorOutput;
    use ort::environment::Environment;
    use ort::session::builder::GraphOptimizationLevel;
    use ort::session::Session;
    use ort::value::Tensor;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    fn environment() -> Result<&'static Environment, String> {
        static ENV: OnceLock<Result<Environment, String>> = OnceLock::new();
        ENV.get_or_init(|| {
            // The library MUST be loaded explicitly through `init_from` first: any other `ort`
            // call would try to load it implicitly and panic on failure.
            let lib = super::runtime_library_path()
                .ok_or_else(|| "Could not resolve the ONNX Runtime library location".to_string())?;
            if !lib.is_file() {
                return Err(format!("ONNX Runtime library not found at {}", lib.display()));
            }
            let builder = std::panic::catch_unwind(|| ort::init_from(&lib))
                .map_err(|_| format!("ONNX Runtime library at {} could not be loaded", lib.display()))?
                .map_err(|e| format!("Failed to load ONNX Runtime library: {e}"))?;
            builder
                .with_name("anycontext")
                .with_telemetry(false)
                .build()
                .map_err(|e| format!("Failed to initialize ONNX Runtime environment: {e}"))
        })
        .as_ref()
        .map_err(|e| e.clone())
    }

    /// Loads the ONNX Runtime library (once) and reports whether it is usable.
    pub fn probe_runtime() -> Result<(), String> {
        environment().map(|_| ())
    }

    /// A loaded ONNX Runtime session bound to the CPU Execution Provider.
    pub struct OnnxSession {
        session: Mutex<Session>,
        input_names: Vec<String>,
        model_id: String,
    }

    impl std::fmt::Debug for OnnxSession {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("OnnxSession")
                .field("model_id", &self.model_id)
                .field("input_names", &self.input_names)
                .finish()
        }
    }

    impl OnnxSession {
        pub fn load(model_path: &Path, model_id: &str, intra_threads: usize) -> Result<Self, String> {
            if !model_path.is_file() {
                return Err(format!("Model file not found at: {}", model_path.display()));
            }
            let env = environment()?;
            let fail = |e: &dyn std::fmt::Display| {
                format!("Failed to load ONNX model '{}' from {}: {e}", model_id, model_path.display())
            };
            let mut builder = Session::builder(env)
                .map_err(|e| fail(&e))?
                .with_optimization_level(GraphOptimizationLevel::Level3)
                .map_err(|e| fail(&e))?
                .with_intra_threads(intra_threads.max(1))
                .map_err(|e| fail(&e))?;
            let session = builder.commit_from_file(model_path).map_err(|e| fail(&e))?;

            let input_names = session.inputs().iter().map(|i| i.name().to_string()).collect();
            Ok(Self {
                session: Mutex::new(session),
                input_names,
                model_id: model_id.to_string(),
            })
        }

        pub fn input_names(&self) -> &[String] {
            &self.input_names
        }

        pub fn has_input(&self, name: &str) -> bool {
            self.input_names.iter().any(|n| n == name)
        }

        pub fn model_id(&self) -> &str {
            &self.model_id
        }

        /// Runs the session with named `i64` tensors sharing one 2-D shape and
        /// returns the first output as a flattened `f32` tensor.
        pub fn run_i64(&self, shape: [usize; 2], inputs: Vec<(String, Vec<i64>)>) -> Result<TensorOutput, String> {
            let mut values = Vec::with_capacity(inputs.len());
            for (name, data) in inputs {
                let tensor = Tensor::from_array(([shape[0], shape[1]], data))
                    .map_err(|e| format!("Failed to build input tensor '{name}': {e}"))?;
                values.push((name, tensor));
            }

            let mut session = self
                .session
                .lock()
                .map_err(|_| format!("ONNX session for '{}' is poisoned", self.model_id))?;
            let outputs = session
                .run(values)
                .map_err(|e| format!("ONNX inference failed for '{}': {e}", self.model_id))?;
            if outputs.len() == 0 {
                return Err(format!("Model '{}' produced no outputs", self.model_id));
            }
            let (out_shape, out_data) = outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| format!("Unexpected output tensor type for '{}': {e}", self.model_id))?;
            Ok(TensorOutput {
                shape: out_shape.iter().copied().collect(),
                data: out_data.to_vec(),
            })
        }
    }
}

#[cfg(not(feature = "runtime"))]
mod imp {
    use super::TensorOutput;
    use std::path::Path;

    pub fn probe_runtime() -> Result<(), String> {
        Err("ONNX Runtime is not linked in this build (feature `runtime` disabled)".to_string())
    }

    /// Placeholder session for builds compiled without the `runtime` feature.
    #[derive(Debug)]
    pub struct OnnxSession {
        _private: (),
    }

    impl OnnxSession {
        pub fn load(_model_path: &Path, model_id: &str, _intra_threads: usize) -> Result<Self, String> {
            Err(format!(
                "ONNX Runtime is not linked in this build (feature `runtime` disabled); '{}' falls back to its deterministic path.",
                model_id
            ))
        }

        pub fn input_names(&self) -> &[String] {
            &[]
        }

        pub fn has_input(&self, _name: &str) -> bool {
            false
        }

        pub fn model_id(&self) -> &str {
            ""
        }

        pub fn run_i64(&self, _shape: [usize; 2], _inputs: Vec<(String, Vec<i64>)>) -> Result<TensorOutput, String> {
            Err("ONNX Runtime is not linked in this build".to_string())
        }
    }
}

pub use imp::{probe_runtime, OnnxSession};

/// Convenience loader using the default CPU thread budget.
pub fn load_session(model_path: &Path, model_id: &str) -> Result<OnnxSession, String> {
    OnnxSession::load(model_path, model_id, default_intra_threads())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_missing_model_is_error_not_panic() {
        let res = load_session(Path::new("definitely_missing_model.onnx"), "missing");
        assert!(res.is_err());
    }

    #[test]
    fn test_load_corrupt_model_is_error_not_panic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("corrupt.onnx");
        let mut bytes = vec![0x08, 0x07, 0x12, 0x04];
        bytes.resize(4096, 0xAB);
        std::fs::write(&path, &bytes).unwrap();
        let res = load_session(&path, "corrupt");
        assert!(res.is_err(), "Garbage bytes must never produce an active session");
    }

    #[test]
    fn test_intra_threads_bounds() {
        let n = default_intra_threads();
        assert!((1..=4).contains(&n));
    }
}
