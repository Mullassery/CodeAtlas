//! Repository discovery: language detection, file walking, and ignore rules.
//!
//! Mirrors upstream's `ProjectScanner` (language detection) and
//! `RepoIgnoreManager` (ignore semantics) using Rust-native, tree-sitter-free
//! suffix mapping and the `ignore` crate for gitwildmatch matching.

use std::collections::BTreeSet;
use std::path::Path;

/// Programming languages CodeAtlas can analyze in the v1 tree-sitter core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Language {
    Python,
    TypeScript,
    JavaScript,
    Go,
    Java,
    Rust,
    Php,
    CSharp,
}

impl Language {
    /// Lowercase identifier matching the upstream `Language` enum values.
    pub fn name(self) -> &'static str {
        match self {
            Language::Python => "python",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Go => "go",
            Language::Java => "java",
            Language::Rust => "rust",
            Language::Php => "php",
            Language::CSharp => "csharp",
        }
    }

    /// Resolve a file suffix (leading dot included, e.g. `.py`) to a language.
    pub fn from_suffix(suffix: &str) -> Option<Language> {
        match suffix {
            ".py" => Some(Language::Python),
            ".ts" | ".tsx" | ".mts" | ".cts" => Some(Language::TypeScript),
            ".js" | ".jsx" | ".mjs" | ".cjs" => Some(Language::JavaScript),
            ".go" => Some(Language::Go),
            ".java" => Some(Language::Java),
            ".rs" => Some(Language::Rust),
            ".php" => Some(Language::Php),
            ".cs" => Some(Language::CSharp),
            _ => None,
        }
    }
}

/// Directory names always excluded regardless of ignore files, mirroring
/// upstream `_ALWAYS_IGNORED_DIRS` (plus the output dir).
const ALWAYS_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".codeboarding",
    "node_modules",
    "__pycache__",
    "build",
    "dist",
    "coverage",
    "target",
];

/// A discovered source file with its detected language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Repo-relative path using forward slashes.
    pub rel_path: String,
    pub language: Language,
}

/// Walk `root`, skipping ignored paths and the output dir, and classify each
/// remaining file by its suffix. Returns files sorted by relative path for
/// deterministic downstream ordering.
pub fn discover_source_files(root: &Path, output_dir: &Path) -> Vec<SourceFile> {
    let mut files: Vec<SourceFile> = Vec::new();

    let walker = ignore::WalkBuilder::new(root)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .hidden(true)
        .parents(true)
        .add_custom_ignore_filename(".codeboardingignore")
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if is_within(path, output_dir) {
            continue;
        }
        if has_always_ignored_component(root, path) {
            continue;
        }
        let Some(suffix) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        let suffix = format!(".{suffix}");
        let Some(language) = Language::from_suffix(&suffix) else {
            continue;
        };
        if let Ok(rel) = path.strip_prefix(root) {
            files.push(SourceFile {
                rel_path: rel.to_string_lossy().replace('\\', "/"),
                language,
            });
        }
    }

    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    files.dedup_by(|a, b| a.rel_path == b.rel_path);
    files
}

/// The set of languages present in a discovery result, ordered.
pub fn present_languages(files: &[SourceFile]) -> BTreeSet<Language> {
    files.iter().map(|f| f.language).collect()
}

/// True if `path` is inside `dir` (path containment).
fn is_within(path: &Path, dir: &Path) -> bool {
    path.starts_with(dir)
}

/// True if any path component is an always-ignored directory name.
fn has_always_ignored_component(root: &Path, path: &Path) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components().any(|c| match c {
        std::path::Component::Normal(part) => part
            .to_str()
            .map(|s| ALWAYS_IGNORED_DIRS.contains(&s))
            .unwrap_or(false),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    // Minimal temp-dir helper (no external crate): create under std::env::temp_dir.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "codeatlas-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&dir).unwrap();
            TempDir(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn suffix_mapping() {
        assert_eq!(Language::from_suffix(".py"), Some(Language::Python));
        assert_eq!(Language::from_suffix(".tsx"), Some(Language::TypeScript));
        assert_eq!(Language::from_suffix(".js"), Some(Language::JavaScript));
        assert_eq!(Language::from_suffix(".go"), Some(Language::Go));
        assert_eq!(Language::from_suffix(".java"), Some(Language::Java));
        assert_eq!(Language::from_suffix(".rs"), Some(Language::Rust));
        assert_eq!(Language::from_suffix(".php"), Some(Language::Php));
        assert_eq!(Language::from_suffix(".cs"), Some(Language::CSharp));
        assert_eq!(Language::from_suffix(".txt"), None);
    }

    #[test]
    fn discovers_files_and_honors_ignores() {
        let tmp = TempDir::new();
        let root = tmp.0.clone();
        fs::create_dir_all(root.join("src/pkg")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::write(root.join("src/a.py"), "x = 1").unwrap();
        fs::write(root.join("src/pkg/b.py"), "y = 2").unwrap();
        fs::write(root.join("src/c.go"), "package c").unwrap();
        fs::write(root.join("notes.txt"), "not code").unwrap();
        fs::write(root.join("node_modules/d.js"), "ignored").unwrap();
        fs::write(root.join(".codeboardingignore"), "src/c.go\n").unwrap();

        let files = discover_source_files(&root, &root.join(".codeboarding"));
        let rels: Vec<&str> = files.iter().map(|f| f.rel_path.as_str()).collect();
        assert_eq!(rels, vec!["src/a.py", "src/pkg/b.py"]);
        assert_eq!(present_languages(&files).len(), 1);
    }
}
