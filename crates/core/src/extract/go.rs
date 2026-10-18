//! Go extraction via tree-sitter.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::{Node, Parser};

use super::{ExtractedFile, Import, UnresolvedCall};
use crate::model::{CallSite, Edge, Node as GraphNode, NodeType};

/// A method receiver: the variable name and base type name.
#[derive(Clone)]
struct Receiver {
    var: String,
    ty: String,
}

#[derive(Clone, PartialEq)]
enum ScopeKind {
    Function,
    Method,
}

struct Parent {
    name: String,
    kind: ScopeKind,
    receiver: Option<Receiver>,
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
        .set_language(&tree_sitter_go::LANGUAGE.into())
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

    collect_definitions(root, source.as_bytes(), &mut ctx);

    let mut parents: Vec<Parent> = Vec::new();
    walk_calls(root, source.as_bytes(), &mut parents, &mut ctx);

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
        "function_declaration" => {
            if let Some(name) = field_name(node, src) {
                let qname = format!("{}.{}", ctx.module, name);
                register_symbol(ctx, &name, &qname);
                ctx.out
                    .nodes
                    .push(graph_node(&qname, node, NodeType::Function));
            }
        }
        "method_declaration" => {
            if let (Some(name), Some((var, ty))) = (field_name(node, src), receiver_of(node, src)) {
                let qname = format!("{}.({}).{}", ctx.module, ty, name);
                register_symbol(ctx, &name, &qname);
                ctx.types
                    .entry(ty.clone())
                    .or_insert_with(|| format!("{}.{}", ctx.module, ty));
                ctx.out
                    .nodes
                    .push(graph_node(&qname, node, NodeType::Method));
                let _ = var;
            }
        }
        "type_declaration" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "type_spec" {
                    register_type(child, src, ctx);
                }
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

fn register_type<'a>(spec: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    let Some(name) = spec
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
    else {
        return;
    };
    let qname = format!("{}.{}", ctx.module, name);
    let node_type = spec
        .child_by_field_name("type")
        .map(|t| {
            if t.kind() == "interface_type" {
                NodeType::Interface
            } else {
                NodeType::Struct
            }
        })
        .unwrap_or(NodeType::Struct);

    ctx.types
        .entry(name.to_string())
        .or_insert_with(|| qname.clone());
    register_symbol(ctx, name, &qname);
    ctx.out.nodes.push(graph_node(&qname, spec, node_type));
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

fn walk_calls<'a>(node: Node<'a>, src: &'a [u8], parents: &mut Vec<Parent>, ctx: &mut Ctx<'a>) {
    match node.kind() {
        "function_declaration" => {
            if let Some(name) = field_name(node, src) {
                parents.push(Parent {
                    name: name.to_string(),
                    kind: ScopeKind::Function,
                    receiver: None,
                });
                walk_body(node, src, parents, ctx);
                parents.pop();
            }
        }
        "method_declaration" => {
            if let Some(name) = field_name(node, src) {
                let receiver = receiver_of(node, src).map(|(var, ty)| Receiver { var, ty });
                parents.push(Parent {
                    name: name.to_string(),
                    kind: ScopeKind::Method,
                    receiver,
                });
                walk_body(node, src, parents, ctx);
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
        "import_declaration" => collect_import(node, src, ctx),
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_calls(child, src, parents, ctx);
            }
        }
    }
}

fn walk_body<'a>(node: Node<'a>, src: &'a [u8], parents: &mut Vec<Parent>, ctx: &mut Ctx<'a>) {
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = node.walk();
        for child in body.named_children(&mut cursor) {
            walk_calls(child, src, parents, ctx);
        }
    }
}

fn handle_call<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let Some(function) = node.child_by_field_name("function") else {
        return;
    };
    let (simple, head) = match function.kind() {
        "identifier" => (text(function, src), None),
        "selector_expression" => {
            let field = function
                .child_by_field_name("field")
                .map(|n| text(n, src))
                .unwrap_or_default();
            let operand = function
                .child_by_field_name("operand")
                .map(|n| text(n, src));
            (field, operand)
        }
        _ => (String::new(), None),
    };

    if simple.is_empty() {
        return;
    }
    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };

    let mut target = None;
    if let Some(head) = head.as_deref() {
        if let Some(recv) = enclosing_receiver(parents) {
            if head == recv.var {
                let q = format!("{}.({}).{}", ctx.module, recv.ty, simple);
                if ctx.known.contains(&q) {
                    target = Some(q);
                }
            }
        }
        if target.is_none() {
            if let Some(base) = head.rsplit('.').next().or(Some(head)) {
                if ctx.types.contains_key(base) {
                    let q = format!("{}.({}).{}", ctx.module, base, simple);
                    if ctx.known.contains(&q) {
                        target = Some(q);
                    }
                }
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

fn enclosing_callable(module: &str, parents: &[Parent]) -> Option<String> {
    let p = parents.last()?;
    Some(match &p.kind {
        ScopeKind::Method => {
            let ty = p.receiver.as_ref().map(|r| r.ty.as_str()).unwrap_or("");
            format!("{module}.({ty}).{}", p.name)
        }
        ScopeKind::Function => format!("{module}.{}", p.name),
    })
}

fn enclosing_receiver(parents: &[Parent]) -> Option<Receiver> {
    parents
        .iter()
        .rev()
        .find(|p| p.kind == ScopeKind::Method)
        .and_then(|p| p.receiver.clone())
}

fn unique_match(ctx: &Ctx, simple: &str) -> Option<String> {
    match ctx.by_simple_name.get(simple) {
        Some(qnames) if qnames.len() == 1 => qnames.first().cloned(),
        _ => None,
    }
}

fn receiver_of<'a>(node: Node<'a>, src: &'a [u8]) -> Option<(String, String)> {
    let receiver = node.child_by_field_name("receiver")?;
    let param = find_first(receiver, "parameter_declaration")?;
    let var = param
        .child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .unwrap_or("")
        .to_string();
    let ty = param
        .child_by_field_name("type")
        .and_then(|n| type_base_name(n, src))
        .unwrap_or_default();
    if ty.is_empty() {
        None
    } else {
        Some((var, ty))
    }
}

fn type_base_name<'a>(node: Node<'a>, src: &'a [u8]) -> Option<String> {
    if node.kind() == "type_identifier"
        || node.kind() == "identifier"
        || node.kind() == "field_identifier"
    {
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

fn find_first<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(found) = find_first(child, kind) {
            return Some(found);
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

fn collect_import<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "import_spec" {
            if let Some(path) = child
                .child_by_field_name("path")
                .and_then(|n| n.utf8_text(src).ok())
            {
                let module = path.trim_matches('"');
                ctx.out.imports.push(Import {
                    module: module.to_string(),
                });
            }
        }
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
    fn extracts_types_functions_and_methods() {
        let src = r#"
package demo

type Service struct {
    name string
}

func New() *Service {
    return &Service{}
}

func (s *Service) Run() {
    s.help()
}

func (s *Service) help() {}
"#;
        let out = extract("demo", src).unwrap();
        assert_eq!(
            qnames(&out),
            vec![
                "demo.(Service).Run",
                "demo.(Service).help",
                "demo.New",
                "demo.Service",
            ]
        );
    }

    #[test]
    fn resolves_receiver_method_calls() {
        let src = "package demo\n\ntype A struct{}\n\nfunc (a *A) One() {\n    a.Two()\n}\n\nfunc (a *A) Two() {}\n";
        let out = extract("demo", src).unwrap();
        assert!(out
            .edges
            .iter()
            .any(|e| e.source == "demo.(A).One" && e.destination == "demo.(A).Two"));
    }
}
