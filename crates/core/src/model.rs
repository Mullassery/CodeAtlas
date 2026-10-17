//! Domain model: the graph and analysis types that mirror the upstream
//! `analysis.json` contract (see `docs/architecture/ADR/0005`).

use serde::{Deserialize, Serialize};

/// LSP `SymbolKind` integer values (1–26), matching upstream `NodeType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NodeType {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

impl NodeType {
    /// Serialized name used by `methods_index` and `file_methods` in upstream.
    pub fn as_str(self) -> &'static str {
        match self {
            NodeType::File => "FILE",
            NodeType::Module => "MODULE",
            NodeType::Namespace => "NAMESPACE",
            NodeType::Package => "PACKAGE",
            NodeType::Class => "CLASS",
            NodeType::Method => "METHOD",
            NodeType::Property => "PROPERTY",
            NodeType::Field => "FIELD",
            NodeType::Constructor => "CONSTRUCTOR",
            NodeType::Enum => "ENUM",
            NodeType::Interface => "INTERFACE",
            NodeType::Function => "FUNCTION",
            NodeType::Variable => "VARIABLE",
            NodeType::Constant => "CONSTANT",
            NodeType::String => "STRING",
            NodeType::Number => "NUMBER",
            NodeType::Boolean => "BOOLEAN",
            NodeType::Array => "ARRAY",
            NodeType::Object => "OBJECT",
            NodeType::Key => "KEY",
            NodeType::Null => "NULL",
            NodeType::EnumMember => "ENUM_MEMBER",
            NodeType::Struct => "STRUCT",
            NodeType::Event => "EVENT",
            NodeType::Operator => "OPERATOR",
            NodeType::TypeParameter => "TYPE_PARAMETER",
        }
    }
}

/// A call-graph node: a named symbol at a source location.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Node {
    pub fully_qualified_name: String,
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    #[serde(default)]
    pub col_start: usize,
    pub node_type: NodeType,
}

/// A single call-site location (one-based line/column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSite {
    pub line: usize,
    pub column: usize,
}

/// A call edge between two qualified names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub source: String,
    pub destination: String,
    #[serde(default)]
    pub call_sites: Vec<CallSite>,
}

/// A non-call reference edge (contains / inherits / typeref / import).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceEdgeKind {
    Contains,
    Inherits,
    Typeref,
    Import,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEdge {
    pub src: String,
    pub dst: String,
    pub kind: ReferenceEdgeKind,
}

/// The merged call graph produced by static analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CallGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub reference_edges: Vec<ReferenceEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_type_names_match_upstream_schema() {
        assert_eq!(NodeType::Function.as_str(), "FUNCTION");
        assert_eq!(NodeType::EnumMember.as_str(), "ENUM_MEMBER");
        assert_eq!(NodeType::Struct.as_str(), "STRUCT");
    }

    #[test]
    fn node_roundtrips_json() {
        let node = Node {
            fully_qualified_name: "pkg.mod.foo".into(),
            file_path: "pkg/mod.py".into(),
            line_start: 1,
            line_end: 3,
            col_start: 0,
            node_type: NodeType::Function,
        };
        let s = serde_json::to_string(&node).unwrap();
        let back: Node = serde_json::from_str(&s).unwrap();
        assert_eq!(node, back);
    }
}
