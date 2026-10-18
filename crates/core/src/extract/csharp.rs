//! C# extraction via tree-sitter.

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
    types: BTreeMap<String, String>,
    class_bases: Vec<(String, String)>,
    known: BTreeSet<String>,
    out: ExtractedFile,
}

pub fn extract<'a>(module: &'a str, source: &'a str) -> Result<ExtractedFile, super::Error> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .map_err(|e| super::Error::Parse(e.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| super::Error::Parse("no tree produced".to_string()))?;

    let root = tree.root_node();
    let module_name = namespace_name(root, source.as_bytes()).unwrap_or_else(|| module.to_string());
    let module: &str = &module_name;
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
    walk_calls(root, source.as_bytes(), &mut parents, &mut ctx);
    resolve_inheritance(&mut ctx);
    finalize(&mut ctx.out);
    ctx.out.class_bases = ctx.class_bases;
    ctx.out.class_bases.sort();
    ctx.out.class_bases.dedup();
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

fn namespace_name(root: Node<'_>, src: &[u8]) -> Option<String> {
    let mut cursor = root.walk();
    for child in root.named_children(&mut cursor) {
        if matches!(
            child.kind(),
            "file_scoped_namespace_declaration" | "namespace_declaration"
        ) {
            return child
                .child_by_field_name("name")
                .and_then(|n| n.utf8_text(src).ok())
                .map(|value| value.replace("::", "."));
        }
    }
    None
}

fn collect_definitions<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    match node.kind() {
        "class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "record_declaration"
        | "struct_declaration" => {
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
        "using_directive" => collect_import(node, src, ctx),
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_definitions(child, src, ctx);
            }
        }
    }
}

fn collect_bases<'a>(node: Node<'a>, src: &'a [u8], class_qname: &str, ctx: &mut Ctx<'a>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "base_list" {
            let mut inner = child.walk();
            for base in child.named_children(&mut inner) {
                if let Some(name) = first_name(base, src) {
                    ctx.class_bases
                        .push((class_qname.to_string(), name.clone()));
                    ctx.out.superclass_names.push(name);
                }
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
                let qname = format!("{}.{}.<init>", ctx.module, class_name);
                register_symbol(ctx, "<init>", &qname);
                ctx.out
                    .nodes
                    .push(graph_node(&qname, child, NodeType::Constructor));
            }
            _ => {}
        }
    }
}

fn walk_calls<'a>(node: Node<'a>, src: &'a [u8], parents: &mut Vec<Parent>, ctx: &mut Ctx<'a>) {
    match node.kind() {
        "namespace_declaration" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "declaration_list" {
                    walk_calls(child, src, parents, ctx);
                }
            }
        }
        "class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "record_declaration"
        | "struct_declaration" => {
            if let Some(name) = field_name(node, src) {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut cursor = body.walk();
                    for child in body.named_children(&mut cursor) {
                        walk_member(child, src, parents, ctx, &name);
                    }
                }
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

fn walk_member<'a>(
    node: Node<'a>,
    src: &'a [u8],
    parents: &mut Vec<Parent>,
    ctx: &mut Ctx<'a>,
    class: &str,
) {
    if matches!(
        node.kind(),
        "method_declaration" | "constructor_declaration"
    ) {
        let name = if node.kind() == "constructor_declaration" {
            "<init>".to_string()
        } else {
            field_name(node, src).unwrap_or_default()
        };
        if !name.is_empty() {
            parents.push(Parent {
                name,
                class: class.to_string(),
            });
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();
                for child in body.named_children(&mut cursor) {
                    walk_member(child, src, parents, ctx, class);
                }
            }
            parents.pop();
        }
    } else if node.kind() == "invocation_expression" {
        handle_call(node, src, parents, ctx);
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk_member(child, src, parents, ctx, class);
        }
    } else if node.kind() == "object_creation_expression" {
        handle_creation(node, src, parents, ctx);
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk_member(child, src, parents, ctx, class);
        }
    } else {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk_member(child, src, parents, ctx, class);
        }
    }
}

fn enclosing_callable(module: &str, parents: &[Parent]) -> Option<String> {
    let parent = parents.last()?;
    Some(format!("{module}.{}.{}", parent.class, parent.name))
}

fn handle_call<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    let (simple, object) = match function.kind() {
        "identifier" => (text(function, src), None),
        "member_access_expression" => (
            function
                .child_by_field_name("name")
                .map(|n| text(n, src))
                .unwrap_or_default(),
            function.child_by_field_name("expression"),
        ),
        _ => return,
    };
    if simple.is_empty() {
        return;
    }
    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };

    let mut target = None;
    let object_text = object.map(|node| text(node, src)).unwrap_or_default();
    if object.is_none() || object_text == "this" {
        if let Some(class) = parents.last().map(|p| p.class.clone()) {
            let q = format!("{}.{}.{}", ctx.module, class, simple);
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
        .and_then(|n| first_name(n, src))
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

fn collect_import<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    if let Some(name) = node.named_child(0).and_then(|n| n.utf8_text(src).ok()) {
        ctx.out.imports.push(Import {
            module: name.to_string(),
        });
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

fn first_name<'a>(node: Node<'a>, src: &'a [u8]) -> Option<String> {
    if matches!(node.kind(), "identifier" | "qualified_name") {
        return node
            .utf8_text(src)
            .ok()
            .map(|value| value.replace("::", "."));
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = first_name(child, src) {
            return Some(name);
        }
    }
    None
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
    fn extracts_namespace_types_methods_and_calls() {
        let src = r#"using App.Base;
namespace App.Demo;
public class Service : Base, IRunnable {
    public Service() {}
    public void Run() { this.Local(); }
    private void Local() {}
}
public interface Api { void Call(); }
public enum Kind { A, B }
public record Point(int X, int Y);
"#;
        let out = extract("ignored", src).unwrap();
        let names = qnames(&out);
        assert!(names.contains(&"App.Demo.Service"));
        assert!(names.contains(&"App.Demo.Service.<init>"));
        assert!(names.contains(&"App.Demo.Service.Run"));
        assert!(names.contains(&"App.Demo.Service.Local"));
        assert!(names.contains(&"App.Demo.Api"));
        assert!(names.contains(&"App.Demo.Kind"));
        assert!(names.contains(&"App.Demo.Point"));
        assert!(out.edges.iter().any(
            |e| e.source == "App.Demo.Service.Run" && e.destination == "App.Demo.Service.Local"
        ));
    }

    #[test]
    fn records_imports_and_inheritance() {
        let src = r#"using App.Base;
namespace App;
class Service : Base, IRunnable { }
class Base { }
"#;
        let out = extract("ignored", src).unwrap();
        assert!(out.imports.iter().any(|i| i.module == "App.Base"));
        assert!(out
            .class_bases
            .iter()
            .any(|(class, base)| class == "App.Service" && base == "Base"));
        assert!(out
            .reference_edges
            .iter()
            .any(|e| e.src == "App.Service" && e.dst == "App.Base"));
    }
}
