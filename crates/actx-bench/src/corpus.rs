//! The benchmark corpus: its documents, their version hash and their plain text.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Extensions whose text is the file itself, so passages can be checked before indexing.
const PLAIN_TEXT: &[&str] = &["md", "txt"];

/// Documents of the corpus, keyed by their path relative to the root with `/` separators.
pub struct Corpus {
    root: PathBuf,
    docs: BTreeMap<String, PathBuf>,
}

impl Corpus {
    pub fn load(root: &Path) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|e| format!("corpus folder '{}' cannot be read: {e}", root.display()))?;
        let mut docs = BTreeMap::new();
        collect(&root, &root, &mut docs)?;
        if docs.is_empty() {
            return Err(format!("corpus folder '{}' has no documents", root.display()));
        }
        Ok(Self { root, docs })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    pub fn contains(&self, doc: &str) -> bool {
        self.docs.contains_key(doc)
    }

    /// The file's text for Markdown and text documents; `None` for the other formats.
    pub fn plain_text(&self, doc: &str) -> Option<String> {
        let path = self.docs.get(doc)?;
        let ext = path.extension()?.to_str()?.to_lowercase();
        PLAIN_TEXT.contains(&ext.as_str()).then(|| std::fs::read_to_string(path).ok()).flatten()
    }

    /// The corpus-relative name of an indexed file path, when the file belongs to the corpus.
    pub fn relative(&self, file_path: &str) -> Option<String> {
        let path = Path::new(file_path);
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let rel = canonical.strip_prefix(&self.root).ok()?;
        let key = to_key(rel);
        self.docs.contains_key(&key).then_some(key)
    }

    /// SHA-256 over the sorted relative paths and file contents.
    pub fn version(&self) -> Result<String, String> {
        let mut hasher = Sha256::new();
        for (key, path) in &self.docs {
            let bytes = std::fs::read(path).map_err(|e| format!("'{}' cannot be read: {e}", path.display()))?;
            hasher.update(key.as_bytes());
            hasher.update([0u8]);
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(&bytes);
        }
        Ok(hex(&hasher.finalize()))
    }
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// Collapses runs of whitespace to one space, so labels do not depend on line wrapping.
pub fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect(root: &Path, dir: &Path, docs: &mut BTreeMap<String, PathBuf>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("'{}' cannot be read: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let hidden = path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'));
        if hidden {
            continue;
        }
        if path.is_dir() {
            collect(root, &path, docs)?;
        } else if path.is_file() {
            let rel = path.strip_prefix(root).map_err(|e| e.to_string())?;
            docs.insert(to_key(rel), path.clone());
        }
    }
    Ok(())
}

fn to_key(rel: &Path) -> String {
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A corpus in a fresh temp folder; the folder is removed when the guard drops.
    pub(crate) fn temp_corpus(files: &[(&str, &str)]) -> (TempDir, Corpus) {
        let dir = TempDir::new();
        for (rel, content) in files {
            let path = dir.0.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, content).unwrap();
        }
        let corpus = Corpus::load(&dir.0).unwrap();
        (dir, corpus)
    }

    pub(crate) struct TempDir(pub PathBuf);

    impl TempDir {
        pub(crate) fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("actx_bench_test_{}_{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn lists_documents_by_relative_path_and_skips_hidden_files() {
        let (_dir, corpus) = temp_corpus(&[("pt/a.md", "a"), ("en/sub/b.txt", "b"), (".hidden", "x")]);
        assert_eq!(corpus.len(), 2);
        assert!(corpus.contains("pt/a.md") && corpus.contains("en/sub/b.txt"));
        assert!(!corpus.contains(".hidden"));
        let abs = corpus.root().join("en").join("sub").join("b.txt");
        assert_eq!(corpus.relative(&abs.to_string_lossy()).as_deref(), Some("en/sub/b.txt"));
        assert_eq!(corpus.relative("/elsewhere/b.txt"), None);
    }

    #[test]
    fn version_changes_with_content_and_names_only() {
        let (_d1, a) = temp_corpus(&[("pt/a.md", "one"), ("pt/b.md", "two")]);
        let (_d2, same) = temp_corpus(&[("pt/b.md", "two"), ("pt/a.md", "one")]);
        let (_d3, edited) = temp_corpus(&[("pt/a.md", "one!"), ("pt/b.md", "two")]);
        let (_d4, renamed) = temp_corpus(&[("pt/c.md", "one"), ("pt/b.md", "two")]);
        let v = a.version().unwrap();
        assert_eq!(v, same.version().unwrap());
        assert_ne!(v, edited.version().unwrap());
        assert_ne!(v, renamed.version().unwrap());
    }

    #[test]
    fn plain_text_only_for_text_formats() {
        let (_dir, corpus) = temp_corpus(&[("a.md", "texto"), ("b.pdf", "%PDF-1.4")]);
        assert_eq!(corpus.plain_text("a.md").as_deref(), Some("texto"));
        assert_eq!(corpus.plain_text("b.pdf"), None);
    }

    #[test]
    fn whitespace_is_normalized() {
        assert_eq!(normalize_whitespace("  a\n\tb   c "), "a b c");
    }
}
