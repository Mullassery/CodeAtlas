//! JavaScript / TypeScript extraction via tree-sitter.
//!
//! Classes, methods, functions (including arrow functions assigned to
//! variables/fields), interfaces, enums, `this.`- and `Class.`-qualified and
//! unique-name call resolution, `new` constructions, `extends` heritage, and
//! imports. `.ts`/`.tsx` are disambiguated by re-parsing with the grammar that
//! yields fewer error nodes.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::{Node, Parser, Tree};

use super::{ExtractedFile, Import};
use crate::discovery::Language;
use crate::model::{CallSite, Edge, Node as GraphNode, NodeType, ReferenceEdge, ReferenceEdgeKind};

#[derive(Clone, Copy, PartialEq)]
enum ScopeKind {
    Class,
    Function,
}

struct Parent {
    name: String,
    kind: ScopeKind,
}

struct Ctx<'a> {
    module: &'a str,
    by_simple_name: BTreeMap<String, Vec<String>>,
    classes: BTreeMap<String, String>,
    known: BTreeSet<String>,
    class_bases: Vec<(String, String)>,
    out: ExtractedFile,
}

/// Extract JS/TS symbols from `source`. `language` selects JavaScript or
/// TypeScript; TypeScript files transparently fall back to the TSX grammar
/// when JSX syntax is present.
pub fn extract<'a>(
    module: &'a str,
    source: &'a str,
    language: Language,
) -> Result<ExtractedFile, super::Error> {
    let tree = match language {
        Language::JavaScript => parse(tree_sitter_javascript::LANGUAGE.into(), source)?,
        _ => parse_typescript(source)?,
    };
    run(module, source, &tree)
}

fn parse(language: tree_sitter::Language, source: &str) -> Result<Tree, super::Error> {
    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .map_err(|e| super::Error::Parse(e.to_string()))?;
    parser
        .parse(source, None)
        .ok_or_else(|| super::Error::Parse("no tree produced".to_string()))
}

/// Parse TypeScript, preferring the `.ts` grammar but falling back to the TSX
/// grammar when JSX syntax makes the plain grammar error out.
fn parse_typescript(source: &str) -> Result<Tree, super::Error> {
    let ts = parse(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), source)?;
    if !ts.root_node().has_error() {
        return Ok(ts);
    }
    let tsx = parse(tree_sitter_typescript::LANGUAGE_TSX.into(), source)?;
    if count_errors(tsx.root_node()) < count_errors(ts.root_node()) {
        Ok(tsx)
    } else {
        Ok(ts)
    }
}

fn count_errors(node: Node) -> usize {
    let mut count = usize::from(node.is_error() || node.is_missing());
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count += count_errors(child);
    }
    count
}

fn run<'a>(module: &'a str, source: &'a str, tree: &Tree) -> Result<ExtractedFile, super::Error> {
    let root = tree.root_node();
    let mut ctx = Ctx {
        module,
        by_simple_name: BTreeMap::new(),
        classes: BTreeMap::new(),
        known: BTreeSet::new(),
        class_bases: Vec::new(),
        out: ExtractedFile::default(),
    };
    let src = source.as_bytes();

    let mut parents: Vec<Parent> = Vec::new();
    collect_definitions(root, src, &mut parents, &mut ctx);

    // Intra-file inheritance edges (base defined in this file).
    let inherits: Vec<ReferenceEdge> = ctx
        .class_bases
        .iter()
        .filter_map(|(class, base)| {
            let base_qname = ctx.classes.get(base)?;
            (base_qname != class).then(|| ReferenceEdge {
                src: class.clone(),
                dst: base_qname.clone(),
                kind: ReferenceEdgeKind::Inherits,
            })
        })
        .collect();
    ctx.out.reference_edges.extend(inherits);

    let mut parents: Vec<Parent> = Vec::new();
    walk_body(root, src, &mut parents, &mut ctx);

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

fn register(ctx: &mut Ctx, name: &str, qname: &str) {
    ctx.by_simple_name
        .entry(name.to_string())
        .or_default()
        .push(qname.to_string());
    ctx.known.insert(qname.to_string());
}

fn collect_definitions<'a>(
    node: Node<'a>,
    src: &'a [u8],
    parents: &mut Vec<Parent>,
    ctx: &mut Ctx<'a>,
) {
    match node.kind() {
        "class_declaration" | "abstract_class_declaration" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let qname = qualify(ctx.module, parents, &name);
            register(ctx, &name, &qname);
            ctx.classes.entry(name.clone()).or_insert(qname.clone());
            ctx.out
                .nodes
                .push(graph_node(&qname, node, NodeType::Class));

            if let Some(enclosing) = enclosing_class_qname(ctx.module, parents) {
                ctx.out.reference_edges.push(ReferenceEdge {
                    src: enclosing,
                    dst: qname.clone(),
                    kind: ReferenceEdgeKind::Contains,
                });
            }
            for base in class_bases(node, src) {
                ctx.class_bases.push((qname.clone(), base));
            }

            parents.push(Parent {
                name,
                kind: ScopeKind::Class,
            });
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = node.walk();
                for child in body.named_children(&mut cursor) {
                    collect_definitions(child, src, parents, ctx);
                }
            }
            parents.pop();
        }
        "interface_declaration" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let qname = qualify(ctx.module, parents, &name);
            register(ctx, &name, &qname);
            ctx.out
                .nodes
                .push(graph_node(&qname, node, NodeType::Interface));
        }
        "enum_declaration" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let qname = qualify(ctx.module, parents, &name);
            register(ctx, &name, &qname);
            ctx.out.nodes.push(graph_node(&qname, node, NodeType::Enum));
        }
        "function_declaration" | "generator_function_declaration" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let qname = qualify(ctx.module, parents, &name);
            register(ctx, &name, &qname);
            ctx.out
                .nodes
                .push(graph_node(&qname, node, NodeType::Function));
            if let Some(enclosing) = enclosing_class_qname(ctx.module, parents) {
                ctx.out.reference_edges.push(ReferenceEdge {
                    src: enclosing,
                    dst: qname.clone(),
                    kind: ReferenceEdgeKind::Contains,
                });
            }
            recurse_body(node, src, parents, ctx);
        }
        "method_definition" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let qname = qualify(ctx.module, parents, &name);
            register(ctx, &name, &qname);
            let node_type = if name == "constructor" {
                NodeType::Constructor
            } else {
                NodeType::Method
            };
            ctx.out.nodes.push(graph_node(&qname, node, node_type));
            if let Some(enclosing) = enclosing_class_qname(ctx.module, parents) {
                ctx.out.reference_edges.push(ReferenceEdge {
                    src: enclosing,
                    dst: qname.clone(),
                    kind: ReferenceEdgeKind::Contains,
                });
            }
            recurse_body(node, src, parents, ctx);
        }
        "field_definition" | "public_field_definition" => {
            // Class field assigned an arrow/function expression is a method.
            if let (Some(name), Some(value)) =
                (field_name(node, src), node.child_by_field_name("value"))
            {
                if is_function_value(value) {
                    let method = parents.last().map(|p| p.kind) == Some(ScopeKind::Class);
                    let qname = qualify(ctx.module, parents, &name);
                    register(ctx, &name, &qname);
                    ctx.out.nodes.push(graph_node(
                        &qname,
                        node,
                        if method {
                            NodeType::Method
                        } else {
                            NodeType::Function
                        },
                    ));
                }
            }
        }
        "variable_declarator" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let Some(value) = node.child_by_field_name("value") else {
                return;
            };
            if !is_function_value(value) {
                return;
            }
            let method = parents.last().map(|p| p.kind) == Some(ScopeKind::Class);
            let qname = qualify(ctx.module, parents, &name);
            register(ctx, &name, &qname);
            ctx.out.nodes.push(graph_node(
                &qname,
                node,
                if method {
                    NodeType::Method
                } else {
                    NodeType::Function
                },
            ));
            if let Some(enclosing) = enclosing_class_qname(ctx.module, parents) {
                ctx.out.reference_edges.push(ReferenceEdge {
                    src: enclosing,
                    dst: qname.clone(),
                    kind: ReferenceEdgeKind::Contains,
                });
            }
        }
        "export_statement" => {
            if let Some(decl) = node.child_by_field_name("declaration") {
                collect_definitions(decl, src, parents, ctx);
            } else {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    collect_definitions(child, src, parents, ctx);
                }
            }
        }
        _ => recurse_definitions(node, src, parents, ctx),
    }
}

fn recurse_definitions<'a>(
    node: Node<'a>,
    src: &'a [u8],
    parents: &mut Vec<Parent>,
    ctx: &mut Ctx<'a>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_definitions(child, src, parents, ctx);
    }
}

fn recurse_body<'a>(node: Node<'a>, src: &'a [u8], parents: &mut Vec<Parent>, ctx: &mut Ctx<'a>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_definitions(child, src, parents, ctx);
    }
}

fn walk_body<'a>(node: Node<'a>, src: &'a [u8], parents: &mut Vec<Parent>, ctx: &mut Ctx<'a>) {
    match node.kind() {
        "class_declaration" | "abstract_class_declaration" | "interface_declaration" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            parents.push(Parent {
                name,
                kind: ScopeKind::Class,
            });
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = node.walk();
                for child in body.named_children(&mut cursor) {
                    walk_body(child, src, parents, ctx);
                }
            }
            parents.pop();
        }
        "function_declaration" | "generator_function_declaration" | "method_definition" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            parents.push(Parent {
                name,
                kind: ScopeKind::Function,
            });
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_body(child, src, parents, ctx);
            }
            parents.pop();
        }
        "variable_declarator" => {
            if let Some(value) = node.child_by_field_name("value") {
                if is_function_value(value)
                    && value.kind() != "identifier"
                    && value.kind() != "member_expression"
                {
                    let name = field_name(node, src).unwrap_or_default();
                    parents.push(Parent {
                        name,
                        kind: ScopeKind::Function,
                    });
                    let mut cursor = value.walk();
                    for child in value.named_children(&mut cursor) {
                        walk_body(child, src, parents, ctx);
                    }
                    parents.pop();
                    return;
                }
            }
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_body(child, src, parents, ctx);
            }
        }
        "call_expression" => {
            handle_call(node, src, parents, ctx);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_body(child, src, parents, ctx);
            }
        }
        "new_expression" => {
            handle_new(node, src, parents, ctx);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_body(child, src, parents, ctx);
            }
        }
        "import_statement" => collect_import(node, src, ctx),
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_body(child, src, parents, ctx);
            }
        }
    }
}

fn handle_call<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let Some(callee) = node.child_by_field_name("function") else {
        return;
    };
    let callee_text = dotted_text(callee, src);
    let simple = simple_name(&callee_text);
    if simple.is_empty() {
        return;
    }
    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };

    let target = resolve_call_target(&callee_text, &simple, parents, ctx);
    push_call(ctx, caller, target, simple, node);
}

fn handle_new<'a>(node: Node<'a>, src: &'a [u8], parents: &[Parent], ctx: &mut Ctx<'a>) {
    let Some(ctor) = node.child_by_field_name("constructor") else {
        return;
    };
    let ctor_text = dotted_text(ctor, src);
    let simple = simple_name(&ctor_text);
    if simple.is_empty() {
        return;
    }
    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };
    let target = ctx.classes.get(&simple).cloned();
    push_call(ctx, caller, target, simple, node);
}

fn push_call<'a>(
    ctx: &mut Ctx<'a>,
    caller: String,
    target: Option<String>,
    simple: String,
    node: Node<'a>,
) {
    match target {
        Some(target) if target != caller => ctx.out.edges.push(Edge {
            source: caller,
            destination: target,
            call_sites: vec![CallSite {
                line: node.start_position().row + 1,
                column: node.start_position().column + 1,
            }],
        }),
        Some(_) => {}
        None => ctx.out.unresolved_calls.push(super::UnresolvedCall {
            caller,
            callee_simple: simple,
            line: node.start_position().row + 1,
            column: node.start_position().column + 1,
        }),
    }
}

fn resolve_call_target(
    callee_text: &str,
    simple: &str,
    parents: &[Parent],
    ctx: &Ctx,
) -> Option<String> {
    // this.method(...) -> enclosing class method.
    if let Some(rest) = callee_text.strip_prefix("this.") {
        if let Some(class) = parents.iter().rev().find(|p| p.kind == ScopeKind::Class) {
            let method_qname = format!("{}.{}", ctx.classes.get(&class.name)?, rest);
            if ctx.known.contains(&method_qname) {
                return Some(method_qname);
            }
        }
    }
    // Class.method(...) / instance.method(...) -> class method.
    if let Some((head, rest)) = callee_text.rsplit_once('.') {
        let base = simple_name(head);
        if let Some(class_qname) = ctx.classes.get(&base) {
            let method_qname = format!("{class_qname}.{rest}");
            if ctx.known.contains(&method_qname) {
                return Some(method_qname);
            }
        }
    }
    // Bare name or fallback: unique simple-name match.
    if let Some(qnames) = ctx.by_simple_name.get(simple) {
        if qnames.len() == 1 {
            return qnames.first().cloned();
        }
    }
    None
}

fn enclosing_callable(module: &str, parents: &[Parent]) -> Option<String> {
    let idx = parents
        .iter()
        .rposition(|p| p.kind == ScopeKind::Function)?;
    let (before, after) = parents.split_at(idx);
    let name = after.first()?.name.clone();
    Some(qualify(module, before, &name))
}

fn enclosing_class_qname(module: &str, parents: &[Parent]) -> Option<String> {
    let idx = parents.iter().rposition(|p| p.kind == ScopeKind::Class)?;
    let (before, after) = parents.split_at(idx);
    let name = after.first()?.name.clone();
    Some(qualify(module, before, &name))
}

fn is_function_value(node: Node) -> bool {
    matches!(
        node.kind(),
        "arrow_function" | "function_expression" | "function" | "generator_function"
    )
}

fn class_bases<'a>(node: Node<'a>, src: &'a [u8]) -> Vec<String> {
    let Some(heritage) = find_child(node, "class_heritage") else {
        return Vec::new();
    };
    // TypeScript wraps the base in an `extends_clause` with a `value` field.
    if let Some(extends) = find_first(heritage, "extends_clause") {
        if let Some(value) = extends.child_by_field_name("value") {
            if let Some(name) = last_identifier(value, src) {
                return vec![name];
            }
        }
    }
    // JavaScript places the base expression directly under `class_heritage`.
    if let Some(base) = heritage.named_child(0) {
        if let Some(name) = last_identifier(base, src) {
            return vec![name];
        }
    }
    Vec::new()
}

fn last_identifier<'a>(node: Node<'a>, src: &'a [u8]) -> Option<String> {
    if matches!(
        node.kind(),
        "identifier" | "type_identifier" | "property_identifier"
    ) {
        return node.utf8_text(src).ok().map(|s| s.to_string());
    }
    let mut last = None;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = last_identifier(child, src) {
            last = Some(name);
        }
    }
    last
}

fn collect_import<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    if let Some(source) = node.child_by_field_name("source") {
        let text = source.utf8_text(src).unwrap_or("");
        let module = text.trim_matches(|c| c == '"' || c == '\'' || c == '`');
        if !module.is_empty() {
            ctx.out.imports.push(Import {
                module: module.to_string(),
            });
        }
    }
}

fn field_name<'a>(node: Node<'a>, src: &'a [u8]) -> Option<String> {
    node.child_by_field_name("name")
        .and_then(|n| n.utf8_text(src).ok())
        .map(|s| s.to_string())
}

fn simple_name(dotted: &str) -> String {
    dotted.rsplit('.').next().unwrap_or(dotted).to_string()
}

/// Join a JS/TS member expression's text (e.g. `this.a.b`).
fn dotted_text<'a>(node: Node<'a>, src: &'a [u8]) -> String {
    match node.kind() {
        "identifier" | "property_identifier" | "private_property_identifier" | "this" => {
            node.utf8_text(src).unwrap_or("").to_string()
        }
        "member_expression" => {
            let obj = node.child_by_field_name("object");
            let prop = node.child_by_field_name("property");
            match (obj, prop) {
                (Some(o), Some(p)) => {
                    let left = dotted_text(o, src);
                    let right = p.utf8_text(src).unwrap_or("");
                    if left.is_empty() {
                        right.to_string()
                    } else {
                        format!("{left}.{right}")
                    }
                }
                _ => String::new(),
            }
        }
        _ => node.utf8_text(src).unwrap_or("").to_string(),
    }
}

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

fn find_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let found = node.named_children(&mut cursor).find(|c| c.kind() == kind);
    found
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

    fn edges(out: &ExtractedFile) -> Vec<(String, String)> {
        out.edges
            .iter()
            .map(|e| (e.source.clone(), e.destination.clone()))
            .collect()
    }

    #[test]
    fn extracts_js_classes_methods_and_functions() {
        let src = r#"
class Animal {
  speak() {
    return this.voice();
  }
  voice() {
    return "...";
  }
}

function run() {
  const a = new Animal();
  a.speak();
}
"#;
        let out = extract("demo", src, Language::JavaScript).unwrap();
        assert_eq!(
            qnames(&out),
            vec![
                "demo.Animal",
                "demo.Animal.speak",
                "demo.Animal.voice",
                "demo.run",
            ]
        );
        assert_eq!(
            edges(&out),
            vec![
                (
                    "demo.Animal.speak".to_string(),
                    "demo.Animal.voice".to_string()
                ),
                ("demo.run".to_string(), "demo.Animal".to_string()),
                ("demo.run".to_string(), "demo.Animal.speak".to_string()),
            ]
        );
    }

    #[test]
    fn extracts_ts_interfaces_and_arrows() {
        let src = r#"
interface Shape {
  area(): number;
}

const square = (n: number): number => n * n;

class Circle implements Shape {
  constructor(private r: number) {}
  area(): number {
    return Math.PI * this.r * this.r;
  }
}

export function main(): void {
  const c = new Circle(2);
  c.area();
  square(3);
}
"#;
        let out = extract("demo", src, Language::TypeScript).unwrap();
        assert!(qnames(&out).contains(&"demo.Shape"));
        assert!(qnames(&out).contains(&"demo.square"));
        assert!(qnames(&out).contains(&"demo.Circle"));
        assert!(qnames(&out).contains(&"demo.Circle.area"));
        assert!(qnames(&out).contains(&"demo.Circle.constructor"));
        assert!(qnames(&out).contains(&"demo.main"));
        assert!(edges(&out).contains(&("demo.main".to_string(), "demo.Circle".to_string())));
        assert!(edges(&out).contains(&("demo.main".to_string(), "demo.Circle.area".to_string())));
        assert!(edges(&out).contains(&("demo.main".to_string(), "demo.square".to_string())));
    }

    #[test]
    fn extracts_jsx_only_in_tsx() {
        let src = r#"
export function Widget() {
  return <div className="x">hi</div>;
}
"#;
        let out = extract("demo", src, Language::TypeScript).unwrap();
        assert!(qnames(&out).contains(&"demo.Widget"));
    }

    #[test]
    fn extracts_imports_and_extends() {
        let src = r#"
import { base } from "./base";
class Dog extends Animal {
  bark() {}
}
"#;
        let out = extract("demo", src, Language::JavaScript).unwrap();
        let mods: Vec<&str> = out.imports.iter().map(|i| i.module.as_str()).collect();
        assert_eq!(mods, vec!["./base"]);
        assert_eq!(
            out.class_bases,
            vec![("demo.Dog".to_string(), "Animal".to_string())]
        );
    }
}
