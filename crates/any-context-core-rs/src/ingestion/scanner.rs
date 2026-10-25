use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use walkdir::{DirEntry, WalkDir};

/// Normalizes path string by removing Windows UNC prefix (`\\?\`) and normalizing separators to OS-native format.
pub fn normalize_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    let clean = if let Some(stripped) = s.strip_prefix(r"\\?\") {
        stripped.to_string()
    } else {
        s.to_string()
    };
    #[cfg(windows)]
    {
        clean.replace('/', "\\")
    }
    #[cfg(not(windows))]
    {
        clean.replace('\\', "/")
    }
}

/// Converts a path to an absolute path string without expanding 8.3 short names on Windows.
pub fn to_absolute_path(path: &Path) -> String {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(path),
            Err(_) => path.to_path_buf(),
        }
    };
    normalize_path(&abs)
}

/// Whether `path` is `folder` or lies inside it, comparing absolute normalized paths on whole
/// path components (`/docs` does not contain `/docs-old/a.md`).
pub fn is_within_folder(path: &str, folder: &str) -> bool {
    let path = to_absolute_path(Path::new(path));
    let folder = to_absolute_path(Path::new(folder));
    // Windows paths are case-insensitive: `C:\Docs` and `c:\docs` are the same folder.
    #[cfg(windows)]
    let (path, folder) = (path.to_lowercase(), folder.to_lowercase());
    let folder = folder.trim_end_matches(['/', '\\']);
    match path.strip_prefix(folder) {
        Some(rest) => rest.is_empty() || rest.starts_with(['/', '\\']),
        None => false,
    }
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

/// Decides from a file's metadata, without opening it, whether it is an online-only cloud
/// placeholder.
pub type OnlineOnlyProbe = Arc<dyn Fn(&Path, &fs::Metadata) -> bool + Send + Sync>;

/// Windows attribute bits of cloud placeholders whose data is not on disk: reading the file
/// makes the cloud client download it (Cloud Files API: OneDrive, Dropbox, iCloud).
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
/// macOS `st_flags` bit of a dataless File Provider file.
const SF_DATALESS: u32 = 0x4000_0000;

pub fn online_only_windows_attributes(attributes: u32) -> bool {
    attributes & (FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS) != 0
}

pub fn online_only_macos_flags(flags: u32) -> bool {
    flags & SF_DATALESS != 0
}

/// Whether a file is an online-only cloud placeholder (ADR-0151). Linux has no placeholder
/// standard, so nothing is online-only there.
#[allow(unused_variables)]
pub fn is_online_only(path: &Path, meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        return online_only_windows_attributes(meta.file_attributes());
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        return online_only_macos_flags(meta.st_flags());
    }
    #[allow(unreachable_code)]
    false
}

#[derive(Clone)]
pub struct WorkspaceScanner {
    online_only: OnlineOnlyProbe,
}

impl Default for WorkspaceScanner {
    fn default() -> Self {
        Self { online_only: Arc::new(is_online_only) }
    }
}

impl std::fmt::Debug for WorkspaceScanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceScanner").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiffResult {
    pub is_up_to_date: bool,
    pub new_files: Vec<String>,
    pub modified_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub renamed_files: Vec<(String, String)>,
    pub disk_files: HashMap<String, (f64, u64)>,
    /// Scanned folders that do not exist or cannot be read at their root.
    pub missing_folders: Vec<String>,
    /// The missing folders plus every directory or file under a scanned folder that could not
    /// be read. Cached files inside them are offline: never listed as deleted (ADR-0150).
    pub offline_paths: Vec<String>,
    /// Online-only cloud placeholders: never read, so never downloaded; a cached one keeps
    /// its index (ADR-0151).
    pub online_only_files: Vec<String>,
}

impl WorkspaceScanner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the online-only placeholder check (tests simulate cloud placeholders with it).
    pub fn with_online_only_probe(mut self, probe: impl Fn(&Path, &fs::Metadata) -> bool + Send + Sync + 'static) -> Self {
        self.online_only = Arc::new(probe);
        self
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
                discovered.push(to_absolute_path(entry.path()));
            }
        }

        discovered
    }
    /// Performs high-speed multi-folder filesystem scan and calculates differential changes
    /// natively in pure Rust against cached metadata (file_path -> (mtime, size)).
    pub fn scan_and_diff_native(
        &self,
        folders: &[String],
        cached_files: &HashMap<String, (f64, u64)>,
    ) -> DiffResult {
        let mut disk_files: HashMap<String, (f64, u64)> = HashMap::new();
        let mut missing_folders: Vec<String> = Vec::new();
        let mut offline_paths: Vec<String> = Vec::new();
        let mut online_only_files: Vec<String> = Vec::new();
        let mut scanned_folders: Vec<String> = Vec::new();

        for folder in folders {
            let root_p = Path::new(folder);
            // A folder that is gone or unreadable is offline, never guessed elsewhere (ADR-0150).
            if fs::read_dir(root_p).is_err() {
                missing_folders.push(folder.to_string());
                offline_paths.push(to_absolute_path(root_p));
                continue;
            }
            scanned_folders.push(to_absolute_path(root_p));

            let walker = WalkDir::new(root_p).into_iter().filter_entry(|e| !is_ignored_dir(e));
            for entry in walker {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(e) => {
                        // A subfolder without permission or an I/O error: its files stay indexed.
                        if let Some(p) = e.path() {
                            offline_paths.push(to_absolute_path(p));
                        }
                        continue;
                    }
                };
                if entry.file_type().is_file() && is_supported_file(entry.path()) {
                    let norm_path = to_absolute_path(entry.path());
                    match entry.metadata() {
                        // Reading a placeholder downloads it: it is listed, never read (ADR-0151).
                        Ok(meta) if (self.online_only)(entry.path(), &meta) => online_only_files.push(norm_path),
                        Ok(meta) => {
                            disk_files.insert(norm_path, (get_file_mtime(&meta), meta.len()));
                        }
                        Err(_) => offline_paths.push(norm_path),
                    }
                }
            }
        }

        let mut norm_cached: HashMap<String, (f64, u64)> = HashMap::with_capacity(cached_files.len());
        for (k, v) in cached_files {
            norm_cached.insert(normalize_path(Path::new(k)), *v);
        }

        let mut new_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut deleted_files = Vec::new();

        for (fp, (d_mtime, d_size)) in &disk_files {
            match norm_cached.get(fp) {
                None => new_files.push(fp.clone()),
                Some((c_mtime, c_size)) => {
                    if (c_mtime - d_mtime).abs() > 0.001 || c_size != d_size {
                        modified_files.push(fp.clone());
                    }
                }
            }
        }

        // Only a file of a folder that was scanned, not under an offline path and not freed to
        // the cloud, is deleted; files of other folders are left to the caller.
        let online_only: std::collections::HashSet<&String> = online_only_files.iter().collect();
        for fp in norm_cached.keys() {
            if !disk_files.contains_key(fp)
                && !online_only.contains(fp)
                && scanned_folders.iter().any(|f| is_within_folder(fp, f))
                && !offline_paths.iter().any(|o| is_within_folder(fp, o))
            {
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
                let size = norm_cached.get(del_f).map(|s| s.1).unwrap_or(0);
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

        let is_up_to_date = new_files.is_empty()
            && modified_files.is_empty()
            && deleted_files.is_empty()
            && renamed_files.is_empty();

        DiffResult {
            is_up_to_date,
            new_files,
            modified_files,
            deleted_files,
            renamed_files,
            disk_files,
            missing_folders,
            offline_paths,
            online_only_files,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn online_only_attribute_bits() {
        assert!(online_only_windows_attributes(0x0040_0000), "recall on data access");
        assert!(online_only_windows_attributes(0x0004_0000 | 0x20), "recall on open");
        assert!(online_only_windows_attributes(0x0000_1000), "offline");
        assert!(!online_only_windows_attributes(0x20), "archive: on disk");
        assert!(!online_only_windows_attributes(0x0008_0020), "pinned: kept on this device");
        assert!(online_only_macos_flags(0x4000_0000), "dataless");
        assert!(!online_only_macos_flags(0), "a file on disk");
    }

    #[test]
    fn is_within_folder_matches_whole_components() {
        assert!(is_within_folder("/work/docs/a.md", "/work/docs"));
        assert!(is_within_folder("/work/docs/sub/b.md", "/work/docs/"));
        assert!(is_within_folder("/work/docs", "/work/docs"));
        assert!(!is_within_folder("/work/docs-old/a.md", "/work/docs"));
        assert!(!is_within_folder("/work/beta/a.md", "/work/docs"));
    }
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
