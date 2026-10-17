//! Tree-sitter based symbol and edge extraction.
//!
//! The extractor is behind a trait so an LSP-backed implementation can replace
//! or augment it later without touching clustering or serialization
//! (ADR-0002). v1 ships a single tree-sitter implementation per language.

pub mod python;

use crate::discovery::Language;
use crate::model::{Edge, Node, ReferenceEdge};

/// An import extracted from a file (a dotted module/package name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub module: String,
}

/// A call site whose target could not be resolved within the file; resolved
/// later against the whole-repo symbol index (unique simple-name heuristic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedCall {
    pub caller: String,
    pub callee_simple: String,
    pub line: usize,
    pub column: usize,
}

/// The per-file result of extraction.
#[derive(Debug, Clone, Default)]
pub struct ExtractedFile {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub reference_edges: Vec<ReferenceEdge>,
    pub imports: Vec<Import>,
    /// Superclass simple names (for cross-file hierarchy resolution).
    pub superclass_names: Vec<String>,
    /// (class qualified name, base simple name) pairs.
    pub class_bases: Vec<(String, String)>,
    /// Calls unresolved within the file, for cross-file resolution.
    pub unresolved_calls: Vec<UnresolvedCall>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no tree-sitter extractor for language {0:?}")]
    Unsupported(Language),
    #[error("tree-sitter failed to parse: {0}")]
    Parse(String),
}

/// Extract symbols and edges from `source` (written in `language`), using
/// `module` as the dotted module path (derived from the file's repo-relative
/// path) when building qualified names.
pub fn extract(language: Language, module: &str, source: &str) -> Result<ExtractedFile, Error> {
    match language {
        Language::Python => python::extract(module, source),
        other => Err(Error::Unsupported(other)),
    }
}

/// Derive the dotted module path from a repo-relative file path
/// (e.g. `pkg/mod.py` -> `pkg.mod`).
pub fn module_path(rel_path: &str) -> String {
    let mut p = rel_path.to_string();
    if let Some(dot) = p.rfind('.') {
        p.truncate(dot);
    }
    p.replace('/', ".")
}
