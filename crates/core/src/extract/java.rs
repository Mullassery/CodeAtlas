//! Java extraction via tree-sitter.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::{Node, Parser};

use super::{ExtractedFile, Import, UnresolvedCall};
use crate::model::{CallSite, Edge, Node as GraphNode, NodeType, ReferenceEdge, ReferenceEdgeKind};

struct Parent {
    name: String,
    class: String,
}

struct Ctx<'a> {
    module: &'a str,
    by_simple_name: BTreeMap<String, Vec<String>>,
    /// Type simple name -> type qualified name.
    types: BTreeMap<String, String>,
    /// (class qualified name, base simple name) pairs for cross-file hierarchy resolution.
    class_bases: Vec<(String, String)>,
    known: BTreeSet<String>,
    out: ExtractedFile,
}

pub fn extract<'a>(module: &'a str, source: &'a str) -> Result<ExtractedFile, super::Error> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|e| super::Error::Parse(e.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| super::Error::Parse("no tree produced".to_string()))?;

    let root = tree.root_node();
    // Java source declares its package; prefer that over the file-derived
    // module (which includes the class filename for `Foo.java`).
    let module = package_name(root, source.as_bytes()).unwrap_or_else(|| module.to_string());
    let module: &str = &module;
    let mut ctx = Ctx {
        module,
        by_simple_name: BTreeMap::new(),
        types: BTreeMap::new(),
        class_bases: Vec::new(),
        known: BTreeSet::new(),
        out: ExtractedFile::default(),
    };

    collect_definitions(root, source.as_bytes(), &mut ctx);

    let mut parents: Vec<Parent> = Vec::new();
    let mut classes: Vec<String> = Vec::new();
    walk_calls(
        root,
        source.as_bytes(),
        &mut parents,
        &mut classes,
        &mut ctx,
    );

    resolve_inheritance(&mut ctx);
    finalize(&mut ctx.out);
    ctx.out.class_bases = ctx.class_bases;
    ctx.out.class_bases.sort();
    ctx.out.class_bases.dedup();
    Ok(ctx.out)
}

fn package_name(root: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if child.kind() == "package_declaration" {
            return child
                .named_child(0)
                .and_then(|n| n.utf8_text(src).ok())
                .map(|s| s.trim().to_string());
        }
    }
    None
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

fn collect_definitions<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    match node.kind() {
        "class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "record_declaration" => {
            if let Some(name) = field_name(node, src) {
                let qname = format!("{}.{}", ctx.module, name);
                let node_type = match node.kind() {
                    "interface_declaration" => NodeType::Interface,
                    "enum_declaration" => NodeType::Enum,
                    _ => NodeType::Class,
                };
                ctx.types
                    .entry(name.clone())
                    .or_insert_with(|| qname.clone());
                register_symbol(ctx, &name, &qname);
                ctx.out.nodes.push(graph_node(&qname, node, node_type));

                collect_bases(node, src, &qname, ctx);
                collect_members(node, src, &name, ctx);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_definitions(child, src, ctx);
            }
        }
    }
}

fn collect_bases<'a>(node: Node<'a>, src: &'a [u8], class_qname: &str, ctx: &mut Ctx<'a>) {
    if let Some(superclass) = node.child_by_field_name("superclass") {
        if let Some(base) = first_type_name(superclass, src) {
            ctx.class_bases
                .push((class_qname.to_string(), base.clone()));
            ctx.out.superclass_names.push(base);
        }
    }
    if let Some(interfaces) = node.child_by_field_name("interfaces") {
        let mut cursor = interfaces.walk();
        for child in interfaces.named_children(&mut cursor) {
            if let Some(base) = first_type_name(child, src) {
                ctx.class_bases
                    .push((class_qname.to_string(), base.clone()));
                ctx.out.superclass_names.push(base);
            }
        }
    }
}

fn collect_members<'a>(node: Node<'a>, src: &'a [u8], class_name: &str, ctx: &mut Ctx<'a>) {
    let Some(body) = node.child_by_field_name("body") else {
        return;
    };
    let mut cursor = body.walk();
    for child in body.named_children(&mut cursor) {
        match child.kind() {
            "method_declaration" => {
                if let Some(name) = field_name(child, src) {
                    let qname = format!("{}.{}.{}", ctx.module, class_name, name);
                    register_symbol(ctx, &name, &qname);
                    ctx.out
                        .nodes
                        .push(graph_node(&qname, child, NodeType::Method));
                }
            }
            "constructor_declaration" => {
                let name = field_name(child, src).unwrap_or_else(|| class_name.to_string());
                let qname = format!("{}.{}.<init>", ctx.module, class_name);
                let _ = name;
                register_symbol(ctx, "<init>", &qname);
                ctx.out
                    .nodes
                    .push(graph_node(&qname, child, NodeType::Constructor));
            }
            _ => {}
        }
    }
}

fn walk_calls<'a>(
    node: Node<'a>,
    src: &'a [u8],
    parents: &mut Vec<Parent>,
    classes: &mut Vec<String>,
    ctx: &mut Ctx<'a>,
) {
    match node.kind() {
        "class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "record_declaration" => {
            if let Some(name) = field_name(node, src) {
                classes.push(name);
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if matches!(child.kind(), "class_body" | "interface_body" | "enum_body") {
                        walk_calls(child, src, parents, classes, ctx);
                    }
                }
                classes.pop();
            }
        }
        "method_declaration" | "constructor_declaration" => {
            let class = classes.last().cloned().unwrap_or_default();
            let name = if node.kind() == "constructor_declaration" {
                "<init>".to_string()
            } else {
                field_name(node, src).unwrap_or_default()
            };
            if !name.is_empty() {
                parents.push(Parent { name, class });
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.named_children(&mut cursor) {
                        walk_calls(child, src, parents, classes, ctx);
                    }
                }
                parents.pop();
            }
        }
        "method_invocation" => {
            handle_call(node, src, parents, ctx);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_calls(child, src, parents, classes, ctx);
            }
        }
        "object_creation_expression" => {
            handle_creation(node, src, parents, ctx);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_calls(child, src, parents, classes, ctx);
            }
        }
        "import_declaration" => collect_import(node, src, ctx),
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_calls(child, src, parents, classes, ctx);
            }
        }
    }
}

fn enclosing_callable(module: &str, parents: &[Parent]) -> Option<String> {
    let p = parents.last()?;
    Some(format!("{module}.{}.{}", p.class, p.name))
}

fn handle_call<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let Some(name_node) = node.child_by_field_name("name") else {
        return;
    };
    let simple = text(name_node, src);
    if simple.is_empty() {
        return;
    }
    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };

    let current_class = parents.last().map(|p| p.class.clone()).unwrap_or_default();
    let object = node.child_by_field_name("object");
    let object_kind = object.map(|n| n.kind()).unwrap_or("");

    let mut target: Option<String> = None;

    if object.is_none() || object_kind == "this" {
        let q = format!("{}.{}.{}", ctx.module, current_class, simple);
        if ctx.known.contains(&q) {
            target = Some(q);
        }
    } else if object_kind == "object_creation_expression" {
        if let Some(ty) = object
            .and_then(|n| n.child_by_field_name("type"))
            .and_then(|n| first_type_name(n, src))
        {
            let q = format!("{}.{}.{}", ctx.module, ty, simple);
            if ctx.known.contains(&q) {
                target = Some(q);
            }
        }
    }

    if target.is_none() {
        target = unique_match(ctx, &simple);
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
            callee_simple: simple,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
        }),
    }
}

fn handle_creation<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };
    let Some(ty) = node
        .child_by_field_name("type")
        .and_then(|n| first_type_name(n, src))
    else {
        return;
    };
    if !ctx.types.contains_key(&ty) {
        return;
    }
    let q = format!("{}.{}.<init>", ctx.module, ty);
    if q != caller && ctx.known.contains(&q) {
        ctx.out.edges.push(Edge {
            source: caller,
            destination: q,
            call_sites: vec![CallSite {
                line: node.start_position().row + 1,
                column: node.start_position().column + 1,
            }],
        });
    }
}

fn unique_match(ctx: &Ctx, simple: &str) -> Option<String> {
    match ctx.by_simple_name.get(simple) {
        Some(qnames) if qnames.len() == 1 => qnames.first().cloned(),
        _ => None,
    }
}

fn resolve_inheritance(ctx: &mut Ctx) {
    let inherits: Vec<ReferenceEdge> = ctx
        .class_bases
        .iter()
        .filter_map(|(class, base)| {
            let base_qname = ctx.types.get(base)?;
            (base_qname != class).then(|| ReferenceEdge {
                src: class.clone(),
                dst: base_qname.clone(),
                kind: ReferenceEdgeKind::Inherits,
            })
        })
        .collect();
    ctx.out.reference_edges.extend(inherits);
}

fn first_type_name<'a>(node: Node<'a>, src: &'a [u8]) -> Option<String> {
    if node.kind() == "type_identifier" || node.kind() == "identifier" {
        return node.utf8_text(src).ok().map(|s| s.to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_type_name(child, src) {
            return Some(name);
        }
    }
    None
}

fn collect_import<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if matches!(child.kind(), "scoped_identifier" | "identifier") {
            let module = child.utf8_text(src).unwrap_or("").trim().to_string();
            if !module.is_empty() {
                ctx.out.imports.push(Import { module });
            }
        }
    }
}

fn register_symbol<'a>(ctx: &mut Ctx<'a>, _name: &str, qname: &str) {
    ctx.known.insert(qname.to_string());
    if let Some(simple) = qname.rsplit('.').next() {
        ctx.by_simple_name
            .entry(simple.to_string())
            .or_default()
            .push(qname.to_string());
    }
}

fn text<'a>(node: Node<'a>, src: &'a [u8]) -> String {
    node.utf8_text(src).unwrap_or("").to_string()
}

fn field_name<'a>(node: Node<'a>, src: &'a [u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(|s| s.to_string())
}

fn graph_node<'a>(qname: &str, node: Node<'a>, node_type: NodeType) -> GraphNode {
    GraphNode {
        fully_qualified_name: qname.to_string(),
        file_path: String::new(),
        line_start: node.start_position().row + 1,
        line_end: node.end_position().row + 1,
        col_start: node.start_position().column + 1,
        node_type,
    }
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

    #[test]
    fn extracts_classes_interfaces_enums_records_and_members() {
        let src = r#"
package com.example.demo;

public class Service {
    public Service() {}
    public void run() {}
    private int helper(int x) { return x; }
}
interface Api { void call(); }
enum Kind { A, B }
record Point(int x, int y) {}
"#;
        let out = extract("com.example.demo", src).unwrap();
        let names = qnames(&out);
        assert!(names.contains(&"com.example.demo.Service"));
        assert!(names.contains(&"com.example.demo.Service.<init>"));
        assert!(names.contains(&"com.example.demo.Service.run"));
        assert!(names.contains(&"com.example.demo.Service.helper"));
        assert!(names.contains(&"com.example.demo.Api"));
        assert!(names.contains(&"com.example.demo.Api.call"));
        assert!(names.contains(&"com.example.demo.Kind"));
        assert!(names.contains(&"com.example.demo.Point"));
    }

    #[test]
    fn resolves_this_and_unqualified_calls() {
        let src = r#"
package demo;
class A {
    void one() { this.two(); three(); }
    void two() {}
    void three() {}
}
"#;
        let out = extract("demo", src).unwrap();
        assert!(out
            .edges
            .iter()
            .any(|e| e.source == "demo.A.one" && e.destination == "demo.A.two"));
        assert!(out
            .edges
            .iter()
            .any(|e| e.source == "demo.A.one" && e.destination == "demo.A.three"));
    }

    #[test]
    fn resolves_constructor_calls_for_known_types() {
        let src = r#"
package demo;
class A { A() {} void run() { new A(); } }
"#;
        let out = extract("demo", src).unwrap();
        assert!(out
            .edges
            .iter()
            .any(|e| e.source == "demo.A.run" && e.destination == "demo.A.<init>"));
    }

    #[test]
    fn records_inheritance_and_imports() {
        let src = r#"
package demo;
import java.util.List;
import static java.util.Collections.emptyList;
class A extends Base implements Runnable, AutoCloseable {
    void run() {}
}
class Base {}
"#;
        let out = extract("demo", src).unwrap();
        assert!(out.imports.iter().any(|i| i.module == "java.util.List"));
        assert!(out
            .class_bases
            .iter()
            .any(|(class, base)| class == "demo.A" && base == "Base"));
        assert!(out
            .class_bases
            .iter()
            .any(|(class, base)| class == "demo.A" && base == "Runnable"));
        assert!(out
            .reference_edges
            .iter()
            .any(|e| e.src == "demo.A" && e.dst == "demo.Base"));
    }
}
