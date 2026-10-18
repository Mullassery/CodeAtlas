//! Deterministic component clustering.
//!
//! v1 implements a directory/name-based hierarchical partition: source files
//! are grouped by directory into a nested component tree (single-child
//! directories are absorbed), limited by `depth_cap`. This is a simplified,
//! deterministic approximation of upstream's name-tree partition + affinity
//! grouper (see `docs/audit/KNOWN_GAPS.md` G1/G9 and ADR-0006); component
//! membership is fully deterministic, while semantic names are left to the
//! Python wrapper's optional LLM step.

use std::collections::BTreeMap;

use crate::analysis::{assign_component_ids, Component, ComponentFileMethodGroup};

/// Directory trie of analyzed files.
#[derive(Default)]
struct Dir {
    files: Vec<String>,
    dirs: BTreeMap<String, Dir>,
}

/// Build the nested component tree from analyzed files and their methods.
///
/// `files` are repo-relative slash paths; `methods_by_file` maps a file path to
/// its sorted list of method qualified names.
pub fn cluster(
    files: &[String],
    methods_by_file: &BTreeMap<String, Vec<String>>,
    depth_cap: usize,
    repo_name: &str,
) -> Vec<Component> {
    let mut root = Dir::default();
    for file in files {
        insert(
            &mut root,
            file,
            file.split('/').collect::<Vec<_>>().as_slice(),
        );
    }

    let mut components = Vec::new();
    for (sub_name, sub) in &root.dirs {
        components.extend(build_scope(sub, sub_name, 1, depth_cap, methods_by_file));
    }
    if !root.files.is_empty() {
        components.push(build_leaf_component(
            repo_name,
            &root.files,
            methods_by_file,
        ));
    }

    assign_component_ids(&mut components, "");
    distinguish_sibling_names(&mut components);
    components
}

/// Rename colliding sibling component names to `"{name} ({component_id})"`,
/// mirroring upstream `distinguish_expanded_component_names`.
fn distinguish_sibling_names(components: &mut [Component]) {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for component in components.iter() {
        *counts.entry(component.name.clone()).or_insert(0) += 1;
    }
    for component in components.iter_mut() {
        if counts.get(&component.name).copied().unwrap_or(0) > 1 {
            component.name = format!("{} ({})", component.name, component.component_id);
        }
        distinguish_sibling_names(&mut component.components);
    }
}

fn insert(dir: &mut Dir, full_path: &str, parts: &[&str]) {
    match parts.split_first() {
        None => {}
        Some((head, tail)) => {
            if tail.is_empty() {
                dir.files.push(full_path.to_string());
            } else {
                let child = dir.dirs.entry((*head).to_string()).or_default();
                insert(child, full_path, tail);
            }
        }
    }
}

/// Build the component(s) for a directory scope, absorbing single-child
/// directories with no files of their own.
fn build_scope(
    dir: &Dir,
    name: &str,
    depth: usize,
    cap: usize,
    methods: &BTreeMap<String, Vec<String>>,
) -> Vec<Component> {
    // Absorb a chain of empty single-child directories, but only below the top
    // level so meaningful top-level boundaries (e.g. `python/` vs `crates/`) are
    // preserved.
    if depth > 1 && dir.files.is_empty() && dir.dirs.len() == 1 {
        let (sub_name, sub) = dir.dirs.iter().next().unwrap();
        return build_scope(sub, sub_name, depth, cap, methods);
    }

    let mut nested = Vec::new();
    if depth < cap {
        for (sub_name, sub) in &dir.dirs {
            nested.extend(build_scope(sub, sub_name, depth + 1, cap, methods));
        }
    }

    let mut component = build_leaf_component(name, &dir.files, methods);
    component.can_expand = !nested.is_empty();
    component.components = nested;
    vec![component]
}

fn build_leaf_component(
    name: &str,
    files: &[String],
    methods: &BTreeMap<String, Vec<String>>,
) -> Component {
    let mut owned = BTreeMap::new();
    let mut symbol_count = 0usize;
    for file in files {
        if let Some(qnames) = methods.get(file) {
            owned.insert(file.clone(), qnames.clone());
            symbol_count += qnames.len();
        }
    }

    let file_methods: Vec<ComponentFileMethodGroup> = owned
        .into_iter()
        .map(|(file_path, methods)| ComponentFileMethodGroup { file_path, methods })
        .collect();

    Component {
        name: name.to_string(),
        description: fallback_description(symbol_count, files.len()),
        key_entities: vec![],
        source_cluster_ids: vec![],
        file_methods,
        component_id: String::new(),
        can_expand: false,
        components: vec![],
        components_relations: vec![],
    }
}

/// A deterministic description used when no LLM naming is available (ADR-0006).
fn fallback_description(symbol_count: usize, file_count: usize) -> String {
    format!("Owns {symbol_count} symbols across {file_count} file(s).")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn methods(files: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        files
            .iter()
            .map(|(f, qs)| (f.to_string(), qs.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    #[test]
    fn clusters_by_directory_with_absorption() {
        let files = vec![
            "src/a.py".to_string(),
            "src/pkg/b.py".to_string(),
            "tests/test_a.py".to_string(),
        ];
        let m = methods(&[
            ("src/a.py", &["src.a.foo"]),
            ("src/pkg/b.py", &["src.pkg.b.bar"]),
            ("tests/test_a.py", &["tests.test_a.test_foo"]),
        ]);

        let components = cluster(&files, &m, 3, "demo");

        assert_eq!(components.len(), 2);
        assert_eq!(components[0].name, "src");
        assert_eq!(components[0].component_id, "1");
        assert_eq!(components[0].components.len(), 1);
        // src/pkg single-child? src has file a.py + dir pkg, so no absorption.
        assert_eq!(components[0].components[0].name, "pkg");
        assert_eq!(components[0].components[0].component_id, "1.1");
        assert_eq!(components[1].name, "tests");
        assert_eq!(components[1].component_id, "2");
    }

    #[test]
    fn absorbs_single_child_directory_chains_below_top() {
        let files = vec!["a/b/c/d.py".to_string()];
        let m = methods(&[("a/b/c/d.py", &["a.b.c.d.foo"])]);
        let components = cluster(&files, &m, 5, "demo");
        // Top-level "a" is preserved; the empty single-child directory "b" is
        // absorbed so "c" (which owns d.py) nests directly under "a".
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].name, "a");
        assert_eq!(components[0].components.len(), 1);
        assert_eq!(components[0].components[0].name, "c");
        assert_eq!(
            components[0].components[0].file_methods[0].file_path,
            "a/b/c/d.py"
        );
    }

    #[test]
    fn depth_cap_limits_expansion() {
        let files = vec![
            "pkg/sub/deep/mod.py".to_string(),
            "pkg/other.py".to_string(),
        ];
        let m = methods(&[
            ("pkg/sub/deep/mod.py", &["pkg.sub.deep.mod.foo"]),
            ("pkg/other.py", &["pkg.other.bar"]),
        ]);
        let components = cluster(&files, &m, 1, "demo");
        // depth cap 1: only "pkg" at top, no expansion into "sub".
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].name, "pkg");
        assert_eq!(components[0].components.len(), 0);
        assert!(!components[0].can_expand);
    }
}
