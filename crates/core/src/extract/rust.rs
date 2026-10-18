//! Rust extraction via tree-sitter.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::{Node, Parser};

use super::{ExtractedFile, Import, UnresolvedCall};
use crate::model::{CallSite, Edge, Node as GraphNode, NodeType};

#[derive(Clone, Copy, PartialEq)]
enum ScopeKind {
    Mod,
    Type,
    Impl,
    Function,
}

struct Parent {
    name: String,
    kind: ScopeKind,
}

struct Ctx<'a> {
    module: &'a str,
    by_simple_name: BTreeMap<String, Vec<String>>,
    /// Type simple name -> type qualified name.
    types: BTreeMap<String, String>,
    known: BTreeSet<String>,
    out: ExtractedFile,
}

pub fn extract<'a>(module: &'a str, source: &'a str) -> Result<ExtractedFile, super::Error> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| super::Error::Parse(e.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| super::Error::Parse("no tree produced".to_string()))?;

    let root = tree.root_node();
    let mut ctx = Ctx {
        module,
        by_simple_name: BTreeMap::new(),
        types: BTreeMap::new(),
        known: BTreeSet::new(),
        out: ExtractedFile::default(),
    };

    let mut parents: Vec<Parent> = Vec::new();
    collect_definitions(root, source.as_bytes(), &mut parents, &mut ctx);

    let mut parents: Vec<Parent> = Vec::new();
    walk_calls(root, source.as_bytes(), &mut parents, &mut ctx);

    collect_imports(root, source.as_bytes(), &mut ctx);

    finalize(&mut ctx.out);
    Ok(ctx.out)
}

fn finalize(out: &mut ExtractedFile) {
    out.nodes
        .sort_by(|a, b| a.fully_qualified_name.cmp(&b.fully_qualified_name));
    out.nodes
        .dedup_by(|a, b| a.fully_qualified_name == b.fully_qualified_name);
    out.edges.sort_by(|a, b| {
        (a.source.as_str(), a.destination.as_str())
            .cmp(&(b.source.as_str(), b.destination.as_str()))
    });
    out.edges.dedup();
    out.reference_edges.sort_by(|a, b| {
        (a.src.as_str(), a.dst.as_str(), a.kind as u8).cmp(&(
            b.src.as_str(),
            b.dst.as_str(),
            b.kind as u8,
        ))
    });
    out.reference_edges.dedup();
    out.imports.sort_by(|a, b| a.module.cmp(&b.module));
    out.imports.dedup();
    out.unresolved_calls.sort_by(|a, b| {
        (
            a.caller.as_str(),
            a.callee_simple.as_str(),
            a.line,
            a.column,
        )
            .cmp(&(
                b.caller.as_str(),
                b.callee_simple.as_str(),
                b.line,
                b.column,
            ))
    });
    out.unresolved_calls.dedup();
}

fn collect_definitions<'a>(
    node: Node<'a>,
    src: &'a [u8],
    parents: &mut Vec<Parent>,
    ctx: &mut Ctx<'a>,
) {
    match node.kind() {
        "struct_item" | "enum_item" | "trait_item" | "union_item" => {
            if let Some(name) = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
            {
                let qname = qualify(ctx.module, parents, name);
                let node_type = match node.kind() {
                    "enum_item" => NodeType::Enum,
                    "trait_item" => NodeType::Interface,
                    _ => NodeType::Struct,
                };
                register_type(ctx, name, &qname);
                register_symbol(ctx, name, &qname);
                ctx.out.nodes.push(GraphNode {
                    fully_qualified_name: qname,
                    file_path: String::new(),
                    line_start: node.start_position().row + 1,
                    line_end: node.end_position().row + 1,
                    col_start: node.start_position().column + 1,
                    node_type,
                });
            }
        }
        "mod_item" => {
            if let Some(name) = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
            {
                parents.push(Parent {
                    name: name.to_string(),
                    kind: ScopeKind::Mod,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = node.walk();
                    for child in body.named_children(&mut cursor) {
                        collect_definitions(child, src, parents, ctx);
                    }
                }
                parents.pop();
            }
        }
        "impl_item" => {
            let target = node
                .child_by_field_name("type")
                .and_then(|n| type_base_name(n, src))
                .unwrap_or_default();
            if !target.is_empty() {
                parents.push(Parent {
                    name: target,
                    kind: ScopeKind::Impl,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = node.walk();
                    for child in body.named_children(&mut cursor) {
                        collect_definitions(child, src, parents, ctx);
                    }
                }
                parents.pop();
            }
        }
        "function_item" => {
            if let Some(name) = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
            {
                let method = parents.last().map(|p| p.kind) == Some(ScopeKind::Impl);
                let qname = qualify(ctx.module, parents, name);
                register_symbol(ctx, name, &qname);
                ctx.out.nodes.push(GraphNode {
                    fully_qualified_name: qname,
                    file_path: String::new(),
                    line_start: node.start_position().row + 1,
                    line_end: node.end_position().row + 1,
                    col_start: node.start_position().column + 1,
                    node_type: if method {
                        NodeType::Method
                    } else {
                        NodeType::Function
                    },
                });

                parents.push(Parent {
                    name: name.to_string(),
                    kind: ScopeKind::Function,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = node.walk();
                    for child in body.named_children(&mut cursor) {
                        collect_definitions(child, src, parents, ctx);
                    }
                }
                parents.pop();
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_definitions(child, src, parents, ctx);
            }
        }
    }
}

fn walk_calls<'a>(node: Node<'a>, src: &'a [u8], parents: &mut Vec<Parent>, ctx: &mut Ctx<'a>) {
    match node.kind() {
        "mod_item" => {
            if let Some(name) = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
            {
                parents.push(Parent {
                    name: name.to_string(),
                    kind: ScopeKind::Mod,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = node.walk();
                    for child in body.named_children(&mut cursor) {
                        walk_calls(child, src, parents, ctx);
                    }
                }
                parents.pop();
            }
        }
        "impl_item" => {
            let target = node
                .child_by_field_name("type")
                .and_then(|n| type_base_name(n, src))
                .unwrap_or_default();
            if !target.is_empty() {
                parents.push(Parent {
                    name: target,
                    kind: ScopeKind::Impl,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = node.walk();
                    for child in body.named_children(&mut cursor) {
                        walk_calls(child, src, parents, ctx);
                    }
                }
                parents.pop();
            }
        }
        "function_item" => {
            if let Some(name) = node
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
            {
                parents.push(Parent {
                    name: name.to_string(),
                    kind: ScopeKind::Function,
                });
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = node.walk();
                    for child in body.named_children(&mut cursor) {
                        walk_calls(child, src, parents, ctx);
                    }
                }
                parents.pop();
            }
        }
        "call_expression" => {
            handle_call(node, src, parents, ctx);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_calls(child, src, parents, ctx);
            }
        }
        "method_call_expression" => {
            handle_method_call(node, src, parents, ctx);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_calls(child, src, parents, ctx);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_calls(child, src, parents, ctx);
            }
        }
    }
}

fn collect_imports<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    if node.kind() == "use_declaration" {
        if let Some(module) = normalize_use(node.utf8_text(src).unwrap_or("")) {
            ctx.out.imports.push(Import { module });
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_imports(child, src, ctx);
    }
}

/// Reduce a `use ...;` declaration to its module path.
///
/// `use a::b::{c, d};` -> `a::b`, `use a::b as c;` -> `a::b`,
/// `use a::b::*;` -> `a::b`, `use serde_json;` -> `serde_json`.
fn normalize_use(raw: &str) -> Option<String> {
    let mut s = raw.trim();
    for kw in ["pub(crate) ", "pub(super) ", "pub(self) ", "pub "] {
        if let Some(rest) = s.strip_prefix(kw) {
            s = rest;
            break;
        }
    }
    let s = s.strip_prefix("use").unwrap_or(s).trim();
    let s = s.strip_suffix(';').unwrap_or(s).trim();
    let s = match s.find('{') {
        Some(i) => &s[..i],
        None => s,
    };
    let s = match s.find(" as ") {
        Some(i) => &s[..i],
        None => s,
    };
    let s = s.trim_start_matches("::").trim();
    let s = s.strip_suffix("::*").unwrap_or(s);
    let s = s.trim_end_matches("::").trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn handle_call<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    let (simple, head) = callee_info(function, src);
    resolve_and_emit(node, &simple, head.as_deref(), parents, ctx);
}

fn handle_method_call<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let simple = node
        .child_by_field_name("field")
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or("")
        .to_string();
    let head = node
        .child_by_field_name("receiver")
        .and_then(|r| r.utf8_text(src).ok())
        .map(|s| s.to_string());
    resolve_and_emit(node, &simple, head.as_deref(), parents, ctx);
}

fn resolve_and_emit<'a>(
    node: Node<'a>,
    simple: &str,
    head: Option<&str>,
    parents: &[Parent],
    ctx: &mut Ctx<'a>,
) {
    if simple.is_empty() {
        return;
    }
    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };

    let mut target = None;
    if head == Some("self") {
        if let Some(owner) = enclosing_owner(ctx.module, parents) {
            let q = format!("{owner}.{simple}");
            if ctx.known.contains(&q) {
                target = Some(q);
            }
        }
    }
    if target.is_none() {
        if let Some(type_name) = head.and_then(simple_type_name) {
            if let Some(tq) = ctx.types.get(type_name) {
                let q = format!("{tq}.{simple}");
                if ctx.known.contains(&q) {
                    target = Some(q);
                }
            }
        }
    }
    if target.is_none() {
        target = unique_match(ctx, simple);
    }

    match target {
        Some(target) if target != caller => {
            ctx.out.edges.push(Edge {
                source: caller,
                destination: target,
                call_sites: vec![CallSite {
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                }],
            });
        }
        Some(_) => {}
        None => ctx.out.unresolved_calls.push(UnresolvedCall {
            caller,
            callee_simple: simple.to_string(),
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
        }),
    }
}

fn unique_match(ctx: &Ctx, simple: &str) -> Option<String> {
    match ctx.by_simple_name.get(simple) {
        Some(qnames) if qnames.len() == 1 => qnames.first().cloned(),
        _ => None,
    }
}

fn simple_type_name(head: &str) -> Option<&str> {
    head.rsplit("::").next().filter(|s| !s.is_empty())
}

/// Extract `(simple, head)` from a call's `function` node.
fn callee_info<'a>(node: Node<'a>, src: &'a [u8]) -> (String, Option<String>) {
    let text = node.utf8_text(src).unwrap_or("").to_string();
    if text.is_empty() {
        return (String::new(), None);
    }
    if let Some((head, simple)) = text.rsplit_once("::") {
        (simple.to_string(), Some(head.to_string()))
    } else if let Some((head, simple)) = text.rsplit_once('.') {
        (simple.to_string(), Some(head.to_string()))
    } else {
        (text, None)
    }
}

fn type_base_name<'a>(node: Node<'a>, src: &'a [u8]) -> Option<String> {
    if is_identifier_kind(node.kind()) {
        return node.utf8_text(src).ok().map(|s| s.to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = type_base_name(child, src) {
            return Some(name);
        }
    }
    None
}

fn is_identifier_kind(kind: &str) -> bool {
    kind == "identifier" || kind == "type_identifier" || kind == "field_identifier"
}

fn register_type<'a>(ctx: &mut Ctx<'a>, name: &'a str, qname: &str) {
    ctx.types
        .entry(name.to_string())
        .or_insert_with(|| qname.to_string());
}

fn register_symbol<'a>(ctx: &mut Ctx<'a>, name: &'a str, qname: &str) {
    ctx.known.insert(qname.to_string());
    ctx.by_simple_name
        .entry(name.to_string())
        .or_default()
        .push(qname.to_string());
}

fn enclosing_callable(module: &str, parents: &[Parent]) -> Option<String> {
    let idx = parents
        .iter()
        .rposition(|p| p.kind == ScopeKind::Function)?;
    let (before, after) = parents.split_at(idx);
    Some(qualify(module, before, &after[0].name))
}

fn enclosing_owner(module: &str, parents: &[Parent]) -> Option<String> {
    let idx = parents
        .iter()
        .rposition(|p| p.kind == ScopeKind::Impl || p.kind == ScopeKind::Type)?;
    let (before, after) = parents.split_at(idx);
    Some(qualify(module, before, &after[0].name))
}

/// Build `module.parents.name`.
fn qualify(module: &str, parents: &[Parent], name: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if !module.is_empty() {
        parts.push(module);
    }
    for p in parents {
        parts.push(&p.name);
    }
    parts.push(name);
    parts.join(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qnames(out: &ExtractedFile) -> Vec<&str> {
        out.nodes
            .iter()
            .map(|n| n.fully_qualified_name.as_str())
            .collect()
    }

    fn edges(out: &ExtractedFile) -> Vec<(String, String)> {
        out.edges
            .iter()
            .map(|e| (e.source.clone(), e.destination.clone()))
            .collect()
    }

    #[test]
    fn extracts_imports() {
        let src = r#"
use serde_json;
use std::collections::{HashMap, BTreeMap};
use crate::engine::Engine as Eng;
use super::sibling::*;
use ::legacy::thing;

fn run() {}
"#;
        let out = extract("demo", src).unwrap();
        let mods: Vec<&str> = out.imports.iter().map(|i| i.module.as_str()).collect();
        assert_eq!(
            mods,
            vec![
                "crate::engine::Engine",
                "legacy::thing",
                "serde_json",
                "std::collections",
                "super::sibling",
            ]
        );
    }

    #[test]
    fn extracts_structs_functions_and_calls() {
        let src = r#"
struct Engine { id: u32 }

impl Engine {
    fn new() -> Engine {
        Engine { id: 0 }
    }

    fn start(&self) {
        self.tick();
    }

    fn tick(&self) {}
}

fn run() -> Engine {
    let e = Engine::new();
    e.start();
    e
}
"#;
        let out = extract("demo", src).unwrap();

        assert_eq!(
            qnames(&out),
            vec![
                "demo.Engine",
                "demo.Engine.new",
                "demo.Engine.start",
                "demo.Engine.tick",
                "demo.run",
            ]
        );

        let edges = edges(&out);
        assert!(edges.contains(&("demo.run".to_string(), "demo.Engine.new".to_string())));
        assert!(edges.contains(&("demo.run".to_string(), "demo.Engine.start".to_string())));
        assert!(edges.contains(&(
            "demo.Engine.start".to_string(),
            "demo.Engine.tick".to_string()
        )));
    }
}
