//! Python extraction via tree-sitter.

use tree_sitter::{Node, Parser};

use super::{ExtractedFile, Import};
use crate::model::{CallSite, Edge, Node as GraphNode, NodeType, ReferenceEdge, ReferenceEdgeKind};

/// The kind of an enclosing definition scope.
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
    /// Qualified-name index: simple name -> fully qualified names.
    by_simple_name: std::collections::BTreeMap<String, Vec<String>>,
    /// Class simple name -> class qualified name.
    classes: std::collections::BTreeMap<String, String>,
    /// The full symbol table of callable/class qualified names.
    known: std::collections::BTreeSet<String>,
    /// (class qualified name, base simple name) pairs for hierarchy resolution.
    class_bases: Vec<(String, String)>,
    out: ExtractedFile,
}

pub fn extract<'a>(module: &'a str, source: &'a str) -> Result<ExtractedFile, super::Error> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|e| super::Error::Parse(e.to_string()))?;
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| super::Error::Parse("no tree produced".to_string()))?;

    let root = tree.root_node();
    let mut ctx = Ctx {
        module,
        by_simple_name: std::collections::BTreeMap::new(),
        classes: std::collections::BTreeMap::new(),
        known: std::collections::BTreeSet::new(),
        class_bases: Vec::new(),
        out: ExtractedFile::default(),
    };

    // First pass: collect definitions into indexes.
    let mut parents: Vec<Parent> = Vec::new();
    collect_definitions(root, source.as_bytes(), &mut parents, &mut ctx);

    // Second pass: walk calls and imports.
    let mut parents: Vec<Parent> = Vec::new();
    walk_body(root, source.as_bytes(), &mut parents, &mut ctx);

    // Resolve intra-file inheritance edges after the class index is complete.
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

    // Sort for deterministic output.
    ctx.out
        .nodes
        .sort_by(|a, b| a.fully_qualified_name.cmp(&b.fully_qualified_name));
    ctx.out
        .nodes
        .dedup_by(|a, b| a.fully_qualified_name == b.fully_qualified_name);
    ctx.out.edges.sort_by(|a, b| {
        (a.source.as_str(), a.destination.as_str())
            .cmp(&(b.source.as_str(), b.destination.as_str()))
    });
    ctx.out.edges.dedup();
    ctx.out.reference_edges.sort_by(|a, b| {
        (a.src.as_str(), a.dst.as_str(), a.kind as u8).cmp(&(
            b.src.as_str(),
            b.dst.as_str(),
            b.kind as u8,
        ))
    });
    ctx.out.reference_edges.dedup();
    ctx.out.imports.sort_by(|a, b| a.module.cmp(&b.module));
    ctx.out.imports.dedup();
    ctx.out.superclass_names.sort();
    ctx.out.superclass_names.dedup();

    Ok(ctx.out)
}

fn collect_definitions<'a>(
    node: Node<'a>,
    src: &'a [u8],
    parents: &mut Vec<Parent>,
    ctx: &mut Ctx<'a>,
) {
    match node.kind() {
        "class_definition" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let qname = qualify(ctx.module, parents, &name);
            ctx.by_simple_name
                .entry(name.clone())
                .or_default()
                .push(qname.clone());
            ctx.classes.entry(name.clone()).or_insert(qname.clone());
            ctx.known.insert(qname.clone());
            ctx.out.nodes.push(GraphNode {
                fully_qualified_name: qname.clone(),
                file_path: String::new(), // set by the caller during merge
                line_start: node.start_position().row + 1,
                line_end: node.end_position().row + 1,
                col_start: node.start_position().column + 1,
                node_type: NodeType::Class,
            });

            // A nested class is contained by its enclosing class.
            if let Some(enclosing) = enclosing_class_qname(ctx.module, parents) {
                ctx.out.reference_edges.push(ReferenceEdge {
                    src: enclosing,
                    dst: qname.clone(),
                    kind: ReferenceEdgeKind::Contains,
                });
            }

            // Superclasses: record both flat names and the class association.
            if let Some(bases) = node.child_by_field_name("superclasses") {
                for base in superclass_names(bases, src) {
                    ctx.out.superclass_names.push(base.clone());
                    ctx.class_bases.push((qname.clone(), base));
                }
            }

            parents.push(Parent {
                name,
                kind: ScopeKind::Class,
            });
            let mut cursor = node.walk();
            let body = node.child_by_field_name("body");
            if let Some(body) = body {
                for child in body.named_children(&mut cursor) {
                    collect_definitions(child, src, parents, ctx);
                }
            }
            parents.pop();
        }
        "function_definition" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            let method = parents.last().map(|p| p.kind) == Some(ScopeKind::Class);
            let qname = qualify(ctx.module, parents, &name);
            ctx.by_simple_name
                .entry(name.clone())
                .or_default()
                .push(qname.clone());
            ctx.known.insert(qname.clone());
            ctx.out.nodes.push(GraphNode {
                fully_qualified_name: qname.clone(),
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

            if let Some(enclosing) = enclosing_class_qname(ctx.module, parents) {
                ctx.out.reference_edges.push(ReferenceEdge {
                    src: enclosing,
                    dst: qname.clone(),
                    kind: ReferenceEdgeKind::Contains,
                });
            }

            parents.push(Parent {
                name,
                kind: ScopeKind::Function,
            });
            let mut cursor = node.walk();
            if let Some(body) = node.child_by_field_name("body") {
                for child in body.named_children(&mut cursor) {
                    collect_definitions(child, src, parents, ctx);
                }
            }
            parents.pop();
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_definitions(child, src, parents, ctx);
            }
        }
    }
}

fn walk_body<'a>(node: Node<'a>, src: &'a [u8], parents: &mut Vec<Parent>, ctx: &mut Ctx<'a>) {
    match node.kind() {
        "class_definition" => {
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
        "function_definition" => {
            let Some(name) = field_name(node, src) else {
                return;
            };
            parents.push(Parent {
                name,
                kind: ScopeKind::Function,
            });
            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = node.walk();
                for child in body.named_children(&mut cursor) {
                    walk_body(child, src, parents, ctx);
                }
            }
            parents.pop();
        }
        "call" => {
            handle_call(node, src, parents, ctx);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_body(child, src, parents, ctx);
            }
        }
        "import_statement" => {
            collect_import(node, src, ctx);
        }
        "import_from_statement" => {
            collect_import_from(node, src, ctx);
        }
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

    let Some(caller) = enclosing_callable(ctx.module, parents) else {
        return;
    };

    let target = resolve_call_target(&callee_text, &simple, parents, ctx);

    if let Some(target) = target {
        if target != caller {
            ctx.out.edges.push(Edge {
                source: caller,
                destination: target,
                call_sites: vec![CallSite {
                    line: node.start_position().row + 1,
                    column: node.start_position().column + 1,
                }],
            });
        }
    }
}

/// The qualified name of the innermost enclosing function/method, if any.
fn enclosing_callable(module: &str, parents: &[Parent]) -> Option<String> {
    let idx = parents
        .iter()
        .rposition(|p| p.kind == ScopeKind::Function)?;
    let (before, after) = parents.split_at(idx);
    let name = after.first()?.name.clone();
    Some(qualify(module, before, &name))
}

/// The qualified name of the innermost enclosing class, if any.
fn enclosing_class_qname(module: &str, parents: &[Parent]) -> Option<String> {
    let idx = parents.iter().rposition(|p| p.kind == ScopeKind::Class)?;
    let (before, after) = parents.split_at(idx);
    let name = after.first()?.name.clone();
    Some(qualify(module, before, &name))
}

fn resolve_call_target(
    callee_text: &str,
    simple: &str,
    parents: &[Parent],
    ctx: &Ctx,
) -> Option<String> {
    // self.method(...) -> enclosing class method.
    if let Some(rest) = callee_text.strip_prefix("self.") {
        if let Some(class) = parents.iter().rev().find(|p| p.kind == ScopeKind::Class) {
            let method_qname = format!("{}.{}", ctx.classes.get(&class.name)?, rest);
            if ctx.known.contains(&method_qname) {
                return Some(method_qname);
            }
        }
    }
    // Class.method(...) / instance.method(...) -> class method.
    if let Some((head, rest)) = callee_text.split_once('.') {
        if let Some(class_qname) = ctx.classes.get(head) {
            let method_qname = format!("{class_qname}.{rest}");
            if ctx.known.contains(&method_qname) {
                return Some(method_qname);
            }
        }
    }
    // Bare name: unique simple-name match (function, method, or class).
    if !callee_text.contains('.') {
        if let Some(qnames) = ctx.by_simple_name.get(simple) {
            if qnames.len() == 1 {
                return qnames.first().cloned();
            }
        }
    }
    // Fallback: unique simple-name match even for dotted callee.
    if let Some(qnames) = ctx.by_simple_name.get(simple) {
        if qnames.len() == 1 {
            return qnames.first().cloned();
        }
    }
    None
}

fn superclass_names<'a>(node: Node<'a>, src: &'a [u8]) -> Vec<String> {
    let mut names = Vec::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "identifier" {
            if let Ok(text) = child.utf8_text(src) {
                names.push(text.to_string());
            }
        } else {
            names.extend(superclass_names(child, src));
        }
    }
    names
}

fn collect_import<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "dotted_name" => push_import(child, src, ctx),
            "aliased_import" => {
                if let Some(name) = child.child_by_field_name("name") {
                    push_import(name, src, ctx);
                }
            }
            _ => {}
        }
    }
}

fn collect_import_from<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    if let Some(module) = node.child_by_field_name("module_name") {
        push_import(module, src, ctx);
    }
}

fn push_import<'a>(node: Node<'a>, src: &'a [u8], ctx: &mut Ctx<'a>) {
    let text = dotted_text(node, src);
    if !text.is_empty() {
        ctx.out.imports.push(Import { module: text });
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

/// Join a dotted expression's text (e.g. `a.b.c`).
fn dotted_text<'a>(node: Node<'a>, src: &'a [u8]) -> String {
    match node.kind() {
        "identifier" => node.utf8_text(src).unwrap_or("").to_string(),
        "attribute" => {
            let obj = node.child_by_field_name("object");
            let attr = node.child_by_field_name("attribute");
            match (obj, attr) {
                (Some(o), Some(a)) => {
                    let left = dotted_text(o, src);
                    let right = a.utf8_text(src).unwrap_or("");
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

/// Build a fully qualified name: `module.parents.name`.
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
    fn extracts_classes_methods_and_calls() {
        let src = r#"
class Animal:
    def speak(self):
        return self.voice()

    def voice(self):
        return "..."

def run():
    a = Animal()
    a.speak()
"#;
        let out = extract("demo", src).unwrap();

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
    fn extracts_hierarchy_and_contains_edges() {
        let src =
            "class Animal:\n    pass\n\nclass Dog(Animal):\n    def bark(self):\n        pass\n";
        let out = extract("demo", src).unwrap();

        let contains: Vec<(String, String)> = out
            .reference_edges
            .iter()
            .filter(|e| e.kind == ReferenceEdgeKind::Contains)
            .map(|e| (e.src.clone(), e.dst.clone()))
            .collect();
        assert!(!contains.contains(&("demo.Animal".to_string(), "demo.Dog".to_string())));
        assert!(contains.contains(&("demo.Dog".to_string(), "demo.Dog.bark".to_string())));

        let inherits: Vec<(String, String)> = out
            .reference_edges
            .iter()
            .filter(|e| e.kind == ReferenceEdgeKind::Inherits)
            .map(|e| (e.src.clone(), e.dst.clone()))
            .collect();
        assert_eq!(
            inherits,
            vec![("demo.Dog".to_string(), "demo.Animal".to_string())]
        );
    }

    #[test]
    fn extracts_imports() {
        let src = "import os\nfrom pkg.sub import thing\nfrom . import sibling\n";
        let out = extract("demo", src).unwrap();
        let mods: Vec<&str> = out.imports.iter().map(|i| i.module.as_str()).collect();
        // Sorted deterministically; relative import yields its raw "." text.
        assert_eq!(mods, vec![".", "os", "pkg.sub"]);
    }
}
