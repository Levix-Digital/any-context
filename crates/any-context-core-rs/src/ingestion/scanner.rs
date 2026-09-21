use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use walkdir::{DirEntry, WalkDir};

/// Normalizes path string by removing Windows UNC prefix (`\\?\`) and normalizing separators.
pub fn normalize_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    let clean = if let Some(stripped) = s.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        s.to_string()
    };
    clean
}

/// Checks if a directory should be skipped from crawling.
pub fn is_ignored_dir(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    let file_name = entry.file_name().to_string_lossy();
    let lower = file_name.to_lowercase();

    // Skip all hidden directories starting with '.' (e.g. .git, .venv, .idea)
    if lower.starts_with('.') && entry.depth() > 0 {
        return true;
    }

    matches!(
        lower.as_str(),
        ".git"
            | ".svn"
            | "node_modules"
            | "__pycache__"
            | ".venv"
            | "venv"
            | "env"
            | ".vs"
            | ".idea"
            | ".vscode"
            | "$recycle.bin"
            | ".tmp"
            | "dist"
            | "build"
            | "out"
            | "target"
            | ".next"
            | ".nuxt"
            | ".turbo"
            | "vendor"
            | "coverage"
            | ".terraform"
    )
}

/// Checks whether a file path has an extension, filename, or pattern supported by AnyContext.
pub fn is_supported_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // 1. Skip temporary lock and OS metadata files
    if file_name.starts_with("~$") || file_name.starts_with("._") {
        return false;
    }

    let fn_lower = file_name.to_lowercase();

    // 2. Skip minified and bundled distribution assets
    if fn_lower.ends_with(".min.js")
        || fn_lower.ends_with(".min.css")
        || fn_lower.ends_with(".bundle.js")
        || fn_lower.ends_with("-min.js")
    {
        return false;
    }

    // 3. Check exact supported filenames or .env.* pattern
    if matches!(
        fn_lower.as_str(),
        "dockerfile" | "containerfile" | "jenkinsfile" | "makefile" | "procfile" | ".env"
    ) || fn_lower.starts_with(".env.")
    {
        return true;
    }

    // 4. Check supported file extension
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(e) => e.to_lowercase(),
        None => return false,
    };

    matches!(
        ext.as_str(),
        // Documents & Text
        "pdf"
            | "docx"
            | "doc"
            | "txt"
            | "text"
            | "md"
            | "rst"
            | "rtf"
            | "odt"
            | "pages"
            | "epub"
            | "eml"
            | "msg"
            | "log"
            // Data & Spreadsheets
            | "csv"
            | "tsv"
            | "json"
            | "jsonl"
            | "xlsx"
            | "xls"
            | "ods"
            | "ofx"
            // Presentations
            | "pptx"
            | "ppt"
            | "key"
            // Code & Tech
            | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "html"
            | "htm"
            | "css"
            | "xml"
            | "yaml"
            | "yml"
            | "toml"
            | "sql"
            | "c"
            | "cpp"
            | "cs"
            | "java"
            | "go"
            | "rs"
            | "sh"
            | "bash"
            | "ps1"
            | "bat"
            | "cmd"
            | "kt"
            | "kts"
            | "swift"
            | "rb"
            | "php"
            | "phtml"
            | "lua"
            | "dart"
            // API Schemas & Contracts
            | "proto"
            | "graphql"
            | "gql"
            | "thrift"
            // Cloud, Configurations & IaC
            | "env"
            | "ini"
            | "cfg"
            | "conf"
            | "properties"
            | "tf"
            | "tfvars"
            | "hcl"
            | "bicep"
            // Build & Dependency Manifests
            | "mod"
            | "sum"
            | "gradle"
            // Images
            | "png"
            | "jpg"
            | "jpeg"
            | "webp"
    )
}

/// Reads file modification time in fractional seconds since UNIX epoch.
pub fn get_file_mtime(metadata: &fs::Metadata) -> f64 {
    metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct WorkspaceScanner;

#[pymethods]
impl WorkspaceScanner {
    #[new]
    pub fn new() -> Self {
        Self
    }

    /// Recursively discovers all supported workspace documents within a root folder.
    pub fn discover_files(&self, root_folder: &str) -> Vec<String> {
        let root_p = Path::new(root_folder);
        if !root_p.exists() {
            return Vec::new();
        }

        let mut discovered = Vec::new();
        let walker = WalkDir::new(root_p).into_iter().filter_entry(|e| !is_ignored_dir(e));

        for entry in walker.filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && is_supported_file(entry.path()) {
                if let Ok(abs) = fs::canonicalize(entry.path()) {
                    discovered.push(normalize_path(&abs));
                } else {
                    discovered.push(normalize_path(entry.path()));
                }
            }
        }

        discovered
    }

    /// Performs high-speed multi-folder filesystem scan and calculates differential changes
    /// against cached metadata from SQLite, including rename detection with zero cost ($0.00).
    pub fn scan_and_diff(
        &self,
        py: Python<'_>,
        folders: Vec<String>,
        cached_files: &Bound<'_, PyDict>,
    ) -> PyResult<PyObject> {
        let mut disk_files: HashMap<String, (f64, u64)> = HashMap::new();

        for folder in &folders {
            let root_p = Path::new(folder);
            if !root_p.exists() {
                continue;
            }

            let walker = WalkDir::new(root_p).into_iter().filter_entry(|e| !is_ignored_dir(e));
            for entry in walker.filter_map(|e| e.ok()) {
                if entry.file_type().is_file() && is_supported_file(entry.path()) {
                    let norm_path = if let Ok(abs) = fs::canonicalize(entry.path()) {
                        normalize_path(&abs)
                    } else {
                        normalize_path(entry.path())
                    };

                    if let Ok(meta) = entry.metadata() {
                        let mtime = get_file_mtime(&meta);
                        let size = meta.len();
                        disk_files.insert(norm_path, (mtime, size));
                    }
                }
            }
        }

        // Read cached files from Python PyDict: path -> {last_mtime: float, file_size: int}
        let mut cached_map: HashMap<String, (f64, u64)> = HashMap::new();
        for (k, v) in cached_files.iter() {
            let path_str: String = k.extract()?;
            if let Ok(dict) = v.downcast::<PyDict>() {
                let mtime: f64 = dict
                    .get_item("last_mtime")?
                    .map(|val| val.extract())
                    .transpose()?
                    .unwrap_or(0.0);
                let size: u64 = dict
                    .get_item("file_size")?
                    .map(|val| val.extract())
                    .transpose()?
                    .unwrap_or(0);
                cached_map.insert(path_str, (mtime, size));
            }
        }

        let mut new_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut deleted_files = Vec::new();

        for (fp, (d_mtime, d_size)) in &disk_files {
            match cached_map.get(fp) {
                None => new_files.push(fp.clone()),
                Some((c_mtime, c_size)) => {
                    if (c_mtime - d_mtime).abs() > 0.001 || c_size != d_size {
                        modified_files.push(fp.clone());
                    }
                }
            }
        }

        for fp in cached_map.keys() {
            if !disk_files.contains_key(fp) {
                deleted_files.push(fp.clone());
            }
        }

        // Detect zero-cost renamed/moved files
        let mut renamed_files: Vec<(String, String)> = Vec::new();
        let mut remaining_new = new_files.clone();
        let mut remaining_deleted = deleted_files.clone();

        for del_f in &deleted_files {
            let (del_size, del_base, del_ext, del_dir) = {
                let p = Path::new(del_f);
                let size = cached_map.get(del_f).map(|s| s.1).unwrap_or(0);
                let base = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                let dir = p.parent().map(normalize_path).unwrap_or_default();
                (size, base, ext, dir)
            };

            let mut matched_new: Option<String> = None;

            for new_f in &remaining_new {
                let (new_size, new_base, new_ext, new_dir) = {
                    let p = Path::new(new_f);
                    let size = disk_files.get(new_f).map(|s| s.1).unwrap_or(0);
                    let base = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                    let dir = p.parent().map(normalize_path).unwrap_or_default();
                    (size, base, ext, dir)
                };

                let is_match = if del_size == new_size && del_size > 0 {
                    if del_base == new_base {
                        true // Moved file across folders
                    } else if del_ext == new_ext && del_dir == new_dir {
                        true // Renamed file in same folder
                    } else {
                        false
                    }
                } else {
                    false
                };

                if is_match {
                    matched_new = Some(new_f.clone());
                    break;
                }
            }

            if let Some(new_f) = matched_new {
                renamed_files.push((del_f.clone(), new_f.clone()));
                remaining_deleted.retain(|f| f != del_f);
                remaining_new.retain(|f| f != &new_f);
            }
        }

        new_files = remaining_new;
        deleted_files = remaining_deleted;

        // Build result dictionary
        let result_dict = PyDict::new_bound(py);
        result_dict.set_item("new_files", PyList::new_bound(py, &new_files))?;
        result_dict.set_item("modified_files", PyList::new_bound(py, &modified_files))?;
        result_dict.set_item("deleted_files", PyList::new_bound(py, &deleted_files))?;

        let py_renamed = PyList::empty_bound(py);
        for (old_p, new_p) in &renamed_files {
            let tuple = PyTuple::new_bound(py, &[old_p.as_str(), new_p.as_str()]);
            py_renamed.append(tuple)?;
        }
        result_dict.set_item("renamed_files", py_renamed)?;
        result_dict.set_item("total_disk_files", disk_files.len())?;

        let py_disk_files = PyDict::new_bound(py);
        for (fp, (mtime, size)) in &disk_files {
            let entry = PyDict::new_bound(py);
            entry.set_item("file_path", fp.as_str())?;
            entry.set_item("last_mtime", *mtime)?;
            entry.set_item("file_size", *size)?;
            py_disk_files.set_item(fp.as_str(), entry)?;
        }
        result_dict.set_item("disk_files", py_disk_files)?;

        Ok(result_dict.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_is_supported_file_extensions() {
        assert!(is_supported_file(Path::new("main.py")));
        assert!(is_supported_file(Path::new("app.tsx")));
        assert!(is_supported_file(Path::new("document.pdf")));
        assert!(is_supported_file(Path::new("table.xlsx")));
        assert!(is_supported_file(Path::new("config.yaml")));
        assert!(is_supported_file(Path::new("schema.graphql")));
        assert!(is_supported_file(Path::new("service.proto")));
        assert!(is_supported_file(Path::new("infra.tf")));
        assert!(is_supported_file(Path::new("Dockerfile")));
        assert!(is_supported_file(Path::new("Makefile")));
        assert!(is_supported_file(Path::new(".env")));
        assert!(is_supported_file(Path::new(".env.local")));
        assert!(is_supported_file(Path::new(".env.production")));
    }

    #[test]
    fn test_is_supported_file_exclusions() {
        assert!(!is_supported_file(Path::new("bundle.min.js")));
        assert!(!is_supported_file(Path::new("styles.min.css")));
        assert!(!is_supported_file(Path::new("app.bundle.js")));
        assert!(!is_supported_file(Path::new("vendor-min.js")));
        assert!(!is_supported_file(Path::new("~$locked.docx")));
        assert!(!is_supported_file(Path::new("._temp")));
        assert!(!is_supported_file(Path::new("binary.exe")));
        assert!(!is_supported_file(Path::new("archive.zip")));
    }

    #[test]
    fn test_discover_files_temp_dir() {
        let temp_dir = std::env::temp_dir().join(format!("actx_scanner_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Valid files
        let py_file = temp_dir.join("test.py");
        let mut f1 = File::create(&py_file).unwrap();
        f1.write_all(b"print('hello')").unwrap();

        let doc_file = temp_dir.join("Dockerfile");
        let mut f2 = File::create(&doc_file).unwrap();
        f2.write_all(b"FROM alpine").unwrap();

        // 2. Ignored file (minified)
        let min_file = temp_dir.join("app.min.js");
        let mut f3 = File::create(&min_file).unwrap();
        f3.write_all(b"var x=1;").unwrap();

        // 3. Ignored directory (.git)
        let git_dir = temp_dir.join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        let git_file = git_dir.join("config");
        let mut f4 = File::create(&git_file).unwrap();
        f4.write_all(b"[core]").unwrap();

        let scanner = WorkspaceScanner::new();
        let discovered = scanner.discover_files(temp_dir.to_str().unwrap());

        let basenames: Vec<String> = discovered
            .iter()
            .map(|p| Path::new(p).file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(basenames.contains(&"test.py".to_string()));
        assert!(basenames.contains(&"Dockerfile".to_string()));
        assert!(!basenames.contains(&"app.min.js".to_string()));
        assert!(!basenames.contains(&"config".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
