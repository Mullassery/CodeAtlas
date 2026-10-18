//! End-to-end deterministic analysis pipeline.
//!
//! Ties together discovery, tree-sitter extraction, cross-file resolution,
//! clustering, and serialization into a single `analyze` entry point that
//! produces the `analysis.json` model and the fingerprint sidecar.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::analysis::{
    method_key, AnalysisMetadata, Component, FileCoverageReport, FileCoverageSummary, FileEntry,
    MethodIndexEntry, NotAnalyzedFile, Relation, RelationCallSite, RelationEdge, UnifiedAnalysis,
};
use crate::clustering::cluster;
use crate::discovery::{discover_all_files, discover_source_files};
use crate::extract::{self, UnresolvedCall};
use crate::fingerprint::{fingerprint, tree_hash};
use crate::model::{CallGraph, Edge, Node, NodeType, ReferenceEdge, ReferenceEdgeKind};

/// Map of repo-relative path -> file entry.
pub type FileIndex = BTreeMap<String, FileEntry>;
/// Map of `"{file}|{qname}"` -> method entry.
pub type MethodIndex = BTreeMap<String, MethodIndexEntry>;
/// A single call-site edge tuple: (source key, target key, line, column).
type ScopedEdge = (String, String, usize, usize);
/// Group call sites by (lca id, source direct-child id, target direct-child id).
type GroupedEdges = BTreeMap<(String, String, String), Vec<ScopedEdge>>;

/// Build per-scope relations: for each cross-component call edge, place the
/// relation at the lowest component that is an ancestor of both endpoints, with
/// endpoints named by the direct children under that ancestor. Returns a map
/// from the LCA component id (`""` for the root scope) to its relations.
fn build_relations(
    graph: &CallGraph,
    qname_to_component: &BTreeMap<String, String>,
    id_to_name: &BTreeMap<String, String>,
    qname_to_key: &BTreeMap<String, String>,
) -> BTreeMap<String, Vec<Relation>> {
    // (lca_id, src_direct_id, dst_direct_id) -> edge (source, target, line, col).
    let mut grouped: GroupedEdges = BTreeMap::new();

    for edge in &graph.edges {
        let Some(src_comp) = qname_to_component.get(&edge.source) else {
            continue;
        };
        let Some(dst_comp) = qname_to_component.get(&edge.destination) else {
            continue;
        };
        if src_comp == dst_comp {
            continue;
        }

        let lca = lca_id(src_comp, dst_comp);
        let lca_depth = if lca.is_empty() { 0 } else { id_depth(&lca) };
        let src_at = truncate_id(src_comp, lca_depth + 1);
        let dst_at = truncate_id(dst_comp, lca_depth + 1);
        if src_at == dst_at {
            continue;
        }

        for site in &edge.call_sites {
            grouped
                .entry((lca.clone(), src_at.clone(), dst_at.clone()))
                .or_default()
                .push((
                    edge.source.clone(),
                    edge.destination.clone(),
                    site.line,
                    site.column,
                ));
        }
    }

    let mut by_scope: BTreeMap<String, Vec<Relation>> = BTreeMap::new();
    for ((lca, src_id, dst_id), edges) in grouped {
        let all_edges = edges
            .into_iter()
            .map(|(source, target, line, column)| RelationEdge {
                source: qname_to_key.get(&source).cloned().unwrap_or(source),
                target: qname_to_key.get(&target).cloned().unwrap_or(target),
                call_sites: vec![RelationCallSite { line, column }],
                description: String::new(),
            })
            .collect();

        by_scope.entry(lca).or_default().push(Relation {
            relation: "calls".to_string(),
            src_name: id_to_name.get(&src_id).cloned().unwrap_or(src_id.clone()),
            dst_name: id_to_name.get(&dst_id).cloned().unwrap_or(dst_id.clone()),
            evidence: String::new(),
            key_edges: vec![],
            src_id,
            dst_id,
            is_static: true,
            all_edges,
        });
    }

    for relations in by_scope.values_mut() {
        relations.sort_by(|a, b| {
            (a.src_id.as_str(), a.dst_id.as_str()).cmp(&(b.src_id.as_str(), b.dst_id.as_str()))
        });
    }

    by_scope
}

/// Assign each component's `components_relations` from the scoped map.
fn assign_relations_to_components(
    components: &mut [Component],
    relations_by_scope: &BTreeMap<String, Vec<Relation>>,
) {
    for component in components.iter_mut() {
        if let Some(relations) = relations_by_scope.get(&component.component_id) {
            component.components_relations = relations.clone();
        }
        assign_relations_to_components(&mut component.components, relations_by_scope);
    }
}

fn id_depth(id: &str) -> usize {
    id.split('.').count()
}

/// Longest common dotted prefix of two hierarchical component ids.
fn lca_id(a: &str, b: &str) -> String {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    let n = a_parts.len().min(b_parts.len());
    let mut i = 0;
    while i < n && a_parts[i] == b_parts[i] {
        i += 1;
    }
    a_parts[..i].join(".")
}

fn truncate_id(id: &str, segments: usize) -> String {
    id.split('.').take(segments).collect::<Vec<_>>().join(".")
}

#[derive(Debug, Clone)]
pub struct AnalyzeOptions {
    pub repo_path: PathBuf,
    pub project_name: String,
    pub depth_cap: usize,
    /// Extra directories (absolute or repo-relative) to exclude from discovery,
    /// on top of the default output directory. Used by `codeatlas-bundle` so the
    /// committed docs directory is not itself analyzed.
    pub ignore_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct AnalyzeResult {
    pub analysis: UnifiedAnalysis,
    pub fingerprint: BTreeMap<String, String>,
    pub file_coverage: FileCoverageReport,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("no source files found in repository")]
    NoSourceFiles,
    #[error("extraction failed for {0}: {1}")]
    Extract(String, extract::Error),
}

/// Files larger than this are not parsed (defense against pathological inputs).
const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;

/// Run a full deterministic analysis of the repository at `repo_path`.
pub fn analyze(opts: &AnalyzeOptions) -> Result<AnalyzeResult, PipelineError> {
    let output_dir = opts.repo_path.join(".codeatlas");
    let mut ignored: Vec<PathBuf> = vec![output_dir];
    for extra in &opts.ignore_paths {
        ignored.push(if extra.is_absolute() {
            extra.clone()
        } else {
            opts.repo_path.join(extra)
        });
    }
    let source_files = discover_source_files(&opts.repo_path, &ignored);
    if source_files.is_empty() {
        return Err(PipelineError::NoSourceFiles);
    }

    let all_files = discover_all_files(&opts.repo_path, &ignored);
    let file_hashes = fingerprint(&source_files, &opts.repo_path);

    // Extract every file and merge into a single graph.
    let mut graph = CallGraph::default();
    let mut class_bases: Vec<(String, String)> = Vec::new();
    let mut unresolved: Vec<UnresolvedCall> = Vec::new();
    let mut analyzed_files: Vec<String> = Vec::new();
    let mut imports_by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for file in &source_files {
        let module = extract::module_path(&file.rel_path);
        let bytes = std::fs::read(opts.repo_path.join(&file.rel_path)).unwrap_or_default();
        if bytes.len() > MAX_FILE_BYTES {
            continue;
        }
        let source = String::from_utf8_lossy(&bytes);
        let extracted = match extract::extract(file.language, &module, &source) {
            Ok(extracted) => extracted,
            Err(extract::Error::Unsupported(_)) => continue,
            Err(e) => return Err(PipelineError::Extract(file.rel_path.clone(), e)),
        };

        analyzed_files.push(file.rel_path.clone());
        for import in &extracted.imports {
            imports_by_file
                .entry(file.rel_path.clone())
                .or_default()
                .push(import.module.clone());
        }
        for mut node in extracted.nodes {
            node.file_path = file.rel_path.clone();
            graph.nodes.push(node);
        }
        graph.edges.extend(extracted.edges);
        graph.reference_edges.extend(extracted.reference_edges);
        class_bases.extend(extracted.class_bases);
        unresolved.extend(extracted.unresolved_calls);
    }

    for modules in imports_by_file.values_mut() {
        modules.sort();
        modules.dedup();
    }

    normalize_nodes(&mut graph.nodes);

    // Cross-file resolution: unique simple-name match for unresolved calls, and
    // inheritance for base classes defined elsewhere.
    let by_simple = index_by_simple_name(&graph.nodes);
    for call in &unresolved {
        if let Some(target) = unique_global_match(&by_simple, &call.callee_simple) {
            graph.edges.push(Edge {
                source: call.caller.clone(),
                destination: target,
                call_sites: vec![crate::model::CallSite {
                    line: call.line,
                    column: call.column,
                }],
            });
        }
    }
    let class_by_simple = index_by_simple_name_filtered(&graph.nodes, NodeType::Class);
    for (class, base) in &class_bases {
        if let Some(base_qname) = unique_global_match(&class_by_simple, base) {
            graph.reference_edges.push(ReferenceEdge {
                src: class.clone(),
                dst: base_qname,
                kind: ReferenceEdgeKind::Inherits,
            });
        }
    }
    normalize_edges(&mut graph);
    // Methods per file (sorted by qualified name) for clustering and indexes.
    let mut methods_by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in &graph.nodes {
        methods_by_file
            .entry(node.file_path.clone())
            .or_default()
            .push(node.fully_qualified_name.clone());
    }
    for qnames in methods_by_file.values_mut() {
        qnames.sort();
        qnames.dedup();
    }

    // Cluster over the full directory tree so real directory boundaries (e.g.
    // a Cargo.toml distinguishing `crates/core` from `crates/py`) are preserved;
    // method membership is attached only for analyzed files.
    let file_paths: Vec<String> = all_files.clone();
    let mut components = cluster(
        &file_paths,
        &methods_by_file,
        opts.depth_cap,
        &opts.project_name,
    );

    let (files, methods_index) = build_indexes(&graph, &file_hashes);
    let qname_to_key: BTreeMap<String, String> = methods_index
        .iter()
        .map(|(key, entry)| (entry.qualified_name.clone(), key.clone()))
        .collect();
    let qname_to_component = build_node_to_component(&components);
    let id_to_name = index_component_names(&components);
    let relations_by_scope =
        build_relations(&graph, &qname_to_component, &id_to_name, &qname_to_key);
    let root_relations = relations_by_scope.get("").cloned().unwrap_or_default();
    assign_relations_to_components(&mut components, &relations_by_scope);

    let depth_level = max_depth(&components);
    let analyzed_set: std::collections::BTreeSet<&str> =
        analyzed_files.iter().map(|s| s.as_str()).collect();
    let source_set: std::collections::BTreeSet<&str> =
        source_files.iter().map(|f| f.rel_path.as_str()).collect();

    let mut not_analyzed_by_reason: BTreeMap<String, usize> = BTreeMap::new();
    let not_analyzed_files: Vec<NotAnalyzedFile> = all_files
        .iter()
        .filter(|f| !analyzed_set.contains(f.as_str()))
        .map(|f| {
            let reason = if source_set.contains(f.as_str()) {
                "unsupported_language"
            } else {
                "other"
            };
            *not_analyzed_by_reason
                .entry(reason.to_string())
                .or_insert(0) += 1;
            NotAnalyzedFile {
                path: f.clone(),
                reason: reason.to_string(),
            }
        })
        .collect();

    let summary = FileCoverageSummary {
        total_files: all_files.len(),
        analyzed: analyzed_files.len(),
        not_analyzed: all_files.len().saturating_sub(analyzed_files.len()),
        not_analyzed_by_reason,
    };
    let generated_at = crate::analysis::utc_now_iso8601();

    let file_coverage = FileCoverageReport {
        version: 1,
        generated_at: generated_at.clone(),
        analyzed_files: analyzed_files.clone(),
        not_analyzed_files,
        imports: imports_by_file,
        summary: summary.clone(),
    };

    let analysis = UnifiedAnalysis {
        metadata: AnalysisMetadata {
            generated_at,
            source_tree_hash: tree_hash(&file_hashes),
            repo_name: opts.project_name.clone(),
            depth_level,
            depth_cap: opts.depth_cap,
            file_coverage_summary: summary,
            tree_spec: serde_json::json!({}),
        },
        description: format!("Architecture map of {}.", opts.project_name),
        files,
        methods_index,
        components,
        components_relations: root_relations,
    };

    Ok(AnalyzeResult {
        analysis,
        fingerprint: file_hashes,
        file_coverage,
    })
}

fn normalize_nodes(nodes: &mut Vec<Node>) {
    nodes.sort_by(|a, b| a.fully_qualified_name.cmp(&b.fully_qualified_name));
    nodes.dedup_by(|a, b| a.fully_qualified_name == b.fully_qualified_name);
}

fn index_by_simple_name(nodes: &[Node]) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in nodes {
        if let Some(simple) = node.fully_qualified_name.rsplit('.').next() {
            map.entry(simple.to_string())
                .or_default()
                .push(node.fully_qualified_name.clone());
        }
    }
    for v in map.values_mut() {
        v.sort();
        v.dedup();
    }
    map
}

fn index_by_simple_name_filtered(nodes: &[Node], kind: NodeType) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in nodes.iter().filter(|n| n.node_type == kind) {
        if let Some(simple) = node.fully_qualified_name.rsplit('.').next() {
            map.entry(simple.to_string())
                .or_default()
                .push(node.fully_qualified_name.clone());
        }
    }
    for v in map.values_mut() {
        v.sort();
        v.dedup();
    }
    map
}

fn unique_global_match(index: &BTreeMap<String, Vec<String>>, simple: &str) -> Option<String> {
    match index.get(simple) {
        Some(qnames) if qnames.len() == 1 => qnames.first().cloned(),
        _ => None,
    }
}

fn normalize_edges(graph: &mut CallGraph) {
    graph.edges.sort_by(|a, b| {
        (a.source.as_str(), a.destination.as_str())
            .cmp(&(b.source.as_str(), b.destination.as_str()))
    });
    graph.edges.dedup();
    graph.reference_edges.sort_by(|a, b| {
        (a.src.as_str(), a.dst.as_str(), a.kind as u8).cmp(&(
            b.src.as_str(),
            b.dst.as_str(),
            b.kind as u8,
        ))
    });
    graph.reference_edges.dedup();
}

fn build_indexes(
    graph: &CallGraph,
    file_hashes: &BTreeMap<String, String>,
) -> (FileIndex, MethodIndex) {
    let mut files: BTreeMap<String, FileEntry> = BTreeMap::new();
    let mut methods_index: BTreeMap<String, MethodIndexEntry> = BTreeMap::new();

    let mut nodes = graph.nodes.clone();
    nodes.sort_by(|a, b| {
        (
            a.file_path.as_str(),
            a.line_start,
            a.line_end,
            a.fully_qualified_name.as_str(),
        )
            .cmp(&(
                b.file_path.as_str(),
                b.line_start,
                b.line_end,
                b.fully_qualified_name.as_str(),
            ))
    });

    for node in &nodes {
        let key = method_key(&node.file_path, &node.fully_qualified_name);
        methods_index.insert(
            key.clone(),
            MethodIndexEntry {
                file_path: node.file_path.clone(),
                qualified_name: node.fully_qualified_name.clone(),
                start_line: node.line_start,
                end_line: node.line_end,
                node_type: node.node_type.as_str().to_string(),
                content_hash: String::new(),
            },
        );
        files
            .entry(node.file_path.clone())
            .or_insert_with(|| FileEntry {
                method_keys: Vec::new(),
                content_hash: file_hashes
                    .get(&node.file_path)
                    .cloned()
                    .unwrap_or_default(),
                module_hash: String::new(),
            })
            .method_keys
            .push(key);
    }

    (files, methods_index)
}

/// Map each method qualified name to its owning (deepest) component id.
fn build_node_to_component(components: &[Component]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut stack: Vec<&Component> = components.iter().collect();
    while let Some(component) = stack.pop() {
        for group in &component.file_methods {
            for method in &group.methods {
                map.insert(method.clone(), component.component_id.clone());
            }
        }
        stack.extend(component.components.iter());
    }
    map
}

fn index_component_names(components: &[Component]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut stack: Vec<&Component> = components.iter().collect();
    while let Some(component) = stack.pop() {
        if !component.component_id.is_empty() {
            map.insert(component.component_id.clone(), component.name.clone());
        }
        stack.extend(component.components.iter());
    }
    map
}

fn max_depth(components: &[Component]) -> usize {
    fn depth(c: &Component) -> usize {
        if c.components.is_empty() {
            1
        } else {
            1 + c.components.iter().map(depth).max().unwrap_or(0)
        }
    }
    components.iter().map(depth).max().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::fs;

    #[test]
    fn analyzes_a_small_repo_end_to_end() {
        let tmp = TempDir::new();
        let root = tmp.path();
        fs::create_dir_all(root.join("app")).unwrap();
        fs::create_dir_all(root.join("core")).unwrap();
        fs::write(root.join("app/main.py"), "def run():\n    return 1\n").unwrap();
        fs::write(
            root.join("core/engine.py"),
            "def start():\n    return run()\n",
        )
        .unwrap();

        let opts = AnalyzeOptions {
            repo_path: root.to_path_buf(),
            project_name: "demo".into(),
            depth_cap: 3,
            ignore_paths: Vec::new(),
        };
        let result = analyze(&opts).unwrap();
        let a = &result.analysis;

        assert_eq!(a.metadata.repo_name, "demo");
        assert!(!a.metadata.source_tree_hash.is_empty());
        assert_eq!(a.components.len(), 2);
        assert_eq!(a.components[0].name, "app");
        assert_eq!(a.components[1].name, "core");

        // Cross-file call: core.engine.start -> app.main.run (unique "run").
        assert!(a.methods_index.contains_key("app/main.py|app.main.run"));
        assert!(a
            .methods_index
            .contains_key("core/engine.py|core.engine.start"));

        assert_eq!(a.components_relations.len(), 1);
        let rel = &a.components_relations[0];
        assert_eq!(rel.src_name, "core");
        assert_eq!(rel.dst_name, "app");
        assert!(rel.is_static);
        assert_eq!(rel.all_edges.len(), 1);
        assert_eq!(rel.all_edges[0].source, "core/engine.py|core.engine.start");
        assert_eq!(rel.all_edges[0].target, "app/main.py|app.main.run");

        // Fingerprint sidecar covers both files.
        assert!(result.fingerprint.contains_key("app/main.py"));
        assert!(result.fingerprint.contains_key("core/engine.py"));
    }

    #[test]
    fn exposes_per_file_imports() {
        let tmp = TempDir::new();
        let root = tmp.path();
        fs::write(
            root.join("app.py"),
            "import os\nimport json\nfrom pkg.sub import thing\n\ndef f():\n    return 1\n",
        )
        .unwrap();

        let opts = AnalyzeOptions {
            repo_path: root.to_path_buf(),
            project_name: "imports".into(),
            depth_cap: 3,
            ignore_paths: Vec::new(),
        };
        let result = analyze(&opts).unwrap();
        let imports = result
            .file_coverage
            .imports
            .get("app.py")
            .expect("imports recorded for analyzed file");
        assert_eq!(
            imports,
            &vec!["json".to_string(), "os".to_string(), "pkg.sub".to_string()]
        );
    }

    #[test]
    fn empty_repo_errors() {
        let tmp = TempDir::new();
        let opts = AnalyzeOptions {
            repo_path: tmp.path().to_path_buf(),
            project_name: "empty".into(),
            depth_cap: 3,
            ignore_paths: Vec::new(),
        };
        assert!(matches!(analyze(&opts), Err(PipelineError::NoSourceFiles)));
    }

    #[test]
    fn oversized_source_is_not_parsed() {
        let tmp = TempDir::new();
        let root = tmp.path();
        let big = "x = 1\n".repeat(2_000_000);
        fs::write(root.join("big.py"), big).unwrap();
        fs::write(root.join("small.py"), "def f():\n    return 1\n").unwrap();

        let opts = AnalyzeOptions {
            repo_path: root.to_path_buf(),
            project_name: "big".into(),
            depth_cap: 3,
            ignore_paths: Vec::new(),
        };
        let result = analyze(&opts).unwrap();
        assert!(result
            .analysis
            .methods_index
            .contains_key("small.py|small.f"));
        assert!(!result
            .analysis
            .methods_index
            .keys()
            .any(|k| k.starts_with("big.py")));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_file_is_skipped() {
        let tmp = TempDir::new();
        let root = tmp.path();
        fs::write(root.join("real.py"), "def f():\n    return 1\n").unwrap();
        let outside =
            std::env::temp_dir().join(format!("codeatlas-outside-{}", std::process::id()));
        fs::write(&outside, "def g():\n    return 2\n").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("link.py")).unwrap();

        let opts = AnalyzeOptions {
            repo_path: root.to_path_buf(),
            project_name: "symlink".into(),
            depth_cap: 3,
            ignore_paths: Vec::new(),
        };
        let result = analyze(&opts).unwrap();
        assert!(result.analysis.methods_index.contains_key("real.py|real.f"));
        assert!(!result
            .analysis
            .methods_index
            .keys()
            .any(|k| k.starts_with("link.py")));

        let _ = fs::remove_file(&outside);
    }

    #[test]
    fn hostile_content_does_not_panic() {
        let tmp = TempDir::new();
        let root = tmp.path();
        fs::write(
            root.join("evil.py"),
            [0xff, 0xfe, 0x00, b'\n', b'x', b'=', b'1', 0x00],
        )
        .unwrap();

        let opts = AnalyzeOptions {
            repo_path: root.to_path_buf(),
            project_name: "hostile".into(),
            depth_cap: 3,
            ignore_paths: Vec::new(),
        };
        // Must not panic on invalid UTF-8 or nonsensical syntax.
        let _ = analyze(&opts);
    }
}
