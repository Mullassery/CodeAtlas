//! Serialization models for the unified `analysis.json` document.
//!
//! Field names and ordering follow the established `analysis.json` contract
//! (see `docs/architecture/ADR/0005`) so downstream readers (web viewer,
//! editors, docs tooling) can consume CodeAtlas output unchanged.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::NodeType;

/// A source-code reference (used by `key_entities` and relation edges).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCodeReference {
    pub qualified_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_start_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_end_line: Option<usize>,
}

/// A one-based source location for a relation edge occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationCallSite {
    pub line: usize,
    pub column: usize,
}

/// A source-to-target code reference supporting a component relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationEdge {
    /// Key into `methods_index` for the source method.
    pub source: String,
    /// Key into `methods_index` for the target method.
    pub target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub call_sites: Vec<RelationCallSite>,
    #[serde(default)]
    pub description: String,
}

/// A relationship between two components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relation {
    pub relation: String,
    pub src_name: String,
    pub dst_name: String,
    #[serde(default)]
    pub evidence: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_edges: Vec<RelationEdge>,
    #[serde(default)]
    pub src_id: String,
    #[serde(default)]
    pub dst_id: String,
    #[serde(default)]
    pub is_static: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub all_edges: Vec<RelationEdge>,
}

/// Component method references grouped by file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentFileMethodGroup {
    pub file_path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<String>,
}

/// A component, possibly with a nested (expanded) sub-analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_entities: Vec<SourceCodeReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_cluster_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_methods: Vec<ComponentFileMethodGroup>,
    #[serde(default)]
    pub component_id: String,
    #[serde(default)]
    pub can_expand: bool,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub components_relations: Vec<Relation>,
}

/// Aggregated file-coverage counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCoverageSummary {
    pub total_files: usize,
    pub analyzed: usize,
    pub not_analyzed: usize,
    pub not_analyzed_by_reason: BTreeMap<String, usize>,
}

/// A persisted file entry; method metadata lives in `methods_index`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileEntry {
    pub method_keys: Vec<String>,
    #[serde(default)]
    pub content_hash: String,
    #[serde(default)]
    pub module_hash: String,
}

/// Canonical method metadata, keyed by `"{file_path}|{qualified_name}"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MethodIndexEntry {
    pub file_path: String,
    pub qualified_name: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default)]
    pub content_hash: String,
}

impl MethodIndexEntry {
    /// Build an entry from a graph node.
    pub fn from_node(
        file_path: impl Into<String>,
        qualified_name: impl Into<String>,
        start_line: usize,
        end_line: usize,
        node_type: NodeType,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            qualified_name: qualified_name.into(),
            start_line,
            end_line,
            node_type: node_type.as_str().to_string(),
            content_hash: content_hash.into(),
        }
    }
}

/// An excluded file with a reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotAnalyzedFile {
    pub path: String,
    pub reason: String,
}

/// The `file_coverage.json` sidecar: which files were analyzed vs. excluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCoverageReport {
    pub version: u32,
    pub generated_at: String,
    pub analyzed_files: Vec<String>,
    pub not_analyzed_files: Vec<NotAnalyzedFile>,
    pub summary: FileCoverageSummary,
}

/// Metadata about the analysis run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub generated_at: String,
    pub source_tree_hash: String,
    pub repo_name: String,
    pub depth_level: usize,
    pub depth_cap: usize,
    #[serde(default)]
    pub file_coverage_summary: FileCoverageSummary,
    #[serde(default)]
    pub tree_spec: serde_json::Value,
}

/// The unified `analysis.json` document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedAnalysis {
    pub metadata: AnalysisMetadata,
    pub description: String,
    pub files: BTreeMap<String, FileEntry>,
    pub methods_index: BTreeMap<String, MethodIndexEntry>,
    pub components: Vec<Component>,
    pub components_relations: Vec<Relation>,
}

impl UnifiedAnalysis {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Canonical `methods_index` key: `"{file_path}|{qualified_name}"`.
pub fn method_key(file_path: &str, qualified_name: &str) -> String {
    format!("{file_path}|{qualified_name}")
}

/// Assign hierarchical component IDs by sibling index (`1`, `1.2`, `1.2.1`).
///
/// Mirrors upstream `assign_component_ids` (non-`only_new` path).
pub fn assign_component_ids(components: &mut [Component], parent_id: &str) {
    for (idx, component) in components.iter_mut().enumerate() {
        let idx = idx + 1;
        component.component_id = if parent_id.is_empty() {
            idx.to_string()
        } else {
            format!("{parent_id}.{idx}")
        };
        assign_component_ids(&mut component.components, &component.component_id);
    }
}

/// Resolve relation endpoints by component name, dropping relations whose
/// endpoint does not name a component in this scope (mirrors `assign_relation_ids`).
pub fn assign_relation_ids(components: &[Component], relations: &mut Vec<Relation>) {
    let name_to_id: BTreeMap<&str, &str> = components
        .iter()
        .filter_map(|c| {
            (!c.component_id.is_empty()).then_some((c.name.as_str(), c.component_id.as_str()))
        })
        .collect();

    relations.retain_mut(|relation| {
        let src_id = name_to_id
            .get(relation.src_name.as_str())
            .copied()
            .unwrap_or("");
        let dst_id = name_to_id
            .get(relation.dst_name.as_str())
            .copied()
            .unwrap_or("");
        if src_id.is_empty() || dst_id.is_empty() {
            return false;
        }
        relation.src_id = src_id.to_string();
        relation.dst_id = dst_id.to_string();
        true
    });
}

/// The current UTC instant formatted as ISO 8601 with microseconds
/// (e.g. `2026-10-15T00:43:22.991714+00:00`), matching upstream's
/// `datetime.now(timezone.utc).isoformat()`.
pub fn utc_now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let micros = now.subsec_micros();

    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = rem / 3600;
    let minute = (rem % 3600) / 60;
    let second = rem % 60;

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{micros:06}+00:00")
}

/// Convert days since the Unix epoch to a (year, month, day) civil date
/// (Howard Hinnant's `civil_from_days` algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_key_joins_with_pipe() {
        assert_eq!(method_key("a/b.py", "a.b.foo"), "a/b.py|a.b.foo");
    }

    #[test]
    fn civil_from_days_known_epoch() {
        // 1970-01-01 is day 0.
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 2000-01-01 is day 10957.
        assert_eq!(civil_from_days(10_957), (2000, 1, 1));
        // 2026-10-15 = 20741 days.
        assert_eq!(civil_from_days(20_741), (2026, 10, 15));
    }

    #[test]
    fn assign_component_ids_is_hierarchical() {
        let mut components = vec![
            Component {
                name: "A".into(),
                description: String::new(),
                key_entities: vec![],
                source_cluster_ids: vec![],
                file_methods: vec![],
                component_id: String::new(),
                can_expand: true,
                components: vec![Component {
                    name: "A1".into(),
                    description: String::new(),
                    key_entities: vec![],
                    source_cluster_ids: vec![],
                    file_methods: vec![],
                    component_id: String::new(),
                    can_expand: false,
                    components: vec![],
                    components_relations: vec![],
                }],
                components_relations: vec![],
            },
            Component {
                name: "B".into(),
                description: String::new(),
                key_entities: vec![],
                source_cluster_ids: vec![],
                file_methods: vec![],
                component_id: String::new(),
                can_expand: false,
                components: vec![],
                components_relations: vec![],
            },
        ];
        assign_component_ids(&mut components, "");
        assert_eq!(components[0].component_id, "1");
        assert_eq!(components[0].components[0].component_id, "1.1");
        assert_eq!(components[1].component_id, "2");
    }

    #[test]
    fn assign_relation_ids_resolves_and_drops() {
        let components = vec![Component {
            name: "Alpha".into(),
            description: String::new(),
            key_entities: vec![],
            source_cluster_ids: vec![],
            file_methods: vec![],
            component_id: "1".into(),
            can_expand: false,
            components: vec![],
            components_relations: vec![],
        }];
        let mut relations = vec![
            Relation {
                relation: "calls".into(),
                src_name: "Alpha".into(),
                dst_name: "Missing".into(),
                evidence: String::new(),
                key_edges: vec![],
                src_id: String::new(),
                dst_id: String::new(),
                is_static: true,
                all_edges: vec![],
            },
            Relation {
                relation: "calls".into(),
                src_name: "Alpha".into(),
                dst_name: "Alpha".into(),
                evidence: String::new(),
                key_edges: vec![],
                src_id: String::new(),
                dst_id: String::new(),
                is_static: false,
                all_edges: vec![],
            },
        ];
        assign_relation_ids(&components, &mut relations);
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].src_id, "1");
        assert_eq!(relations[0].dst_id, "1");
    }

    #[test]
    fn serializes_upstream_shape() {
        let analysis = UnifiedAnalysis {
            metadata: AnalysisMetadata {
                generated_at: "2026-10-15T00:43:22.991714+00:00".into(),
                source_tree_hash: "9bc75e70".into(),
                repo_name: "demo".into(),
                depth_level: 1,
                depth_cap: 3,
                file_coverage_summary: FileCoverageSummary {
                    total_files: 2,
                    analyzed: 1,
                    not_analyzed: 1,
                    not_analyzed_by_reason: BTreeMap::from([("other".to_string(), 1)]),
                },
                tree_spec: serde_json::json!({}),
            },
            description: "A demo project.".into(),
            files: BTreeMap::from([(
                "mod.py".to_string(),
                FileEntry {
                    method_keys: vec!["mod.py|mod.foo".to_string()],
                    content_hash: "abc123".into(),
                    module_hash: "def456".into(),
                },
            )]),
            methods_index: BTreeMap::from([(
                "mod.py|mod.foo".to_string(),
                MethodIndexEntry {
                    file_path: "mod.py".into(),
                    qualified_name: "mod.foo".into(),
                    start_line: 1,
                    end_line: 3,
                    node_type: "FUNCTION".into(),
                    content_hash: "abc123".into(),
                },
            )]),
            components: vec![Component {
                name: "Core".into(),
                description: "Core logic".into(),
                key_entities: vec![SourceCodeReference {
                    qualified_name: "mod.foo".into(),
                    reference_file: Some("mod.py".into()),
                    reference_start_line: Some(1),
                    reference_end_line: Some(3),
                }],
                source_cluster_ids: vec!["1.0".into()],
                file_methods: vec![ComponentFileMethodGroup {
                    file_path: "mod.py".into(),
                    methods: vec!["mod.foo".into()],
                }],
                component_id: "1".into(),
                can_expand: false,
                components: vec![],
                components_relations: vec![],
            }],
            components_relations: vec![],
        };

        let json = analysis.to_json_pretty().unwrap();

        let expected = r#"{
  "metadata": {
    "generated_at": "2026-10-15T00:43:22.991714+00:00",
    "source_tree_hash": "9bc75e70",
    "repo_name": "demo",
    "depth_level": 1,
    "depth_cap": 3,
    "file_coverage_summary": {
      "total_files": 2,
      "analyzed": 1,
      "not_analyzed": 1,
      "not_analyzed_by_reason": {
        "other": 1
      }
    },
    "tree_spec": {}
  },
  "description": "A demo project.",
  "files": {
    "mod.py": {
      "method_keys": [
        "mod.py|mod.foo"
      ],
      "content_hash": "abc123",
      "module_hash": "def456"
    }
  },
  "methods_index": {
    "mod.py|mod.foo": {
      "file_path": "mod.py",
      "qualified_name": "mod.foo",
      "start_line": 1,
      "end_line": 3,
      "type": "FUNCTION",
      "content_hash": "abc123"
    }
  },
  "components": [
    {
      "name": "Core",
      "description": "Core logic",
      "key_entities": [
        {
          "qualified_name": "mod.foo",
          "reference_file": "mod.py",
          "reference_start_line": 1,
          "reference_end_line": 3
        }
      ],
      "source_cluster_ids": [
        "1.0"
      ],
      "file_methods": [
        {
          "file_path": "mod.py",
          "methods": [
            "mod.foo"
          ]
        }
      ],
      "component_id": "1",
      "can_expand": false,
      "components": [],
      "components_relations": []
    }
  ],
  "components_relations": []
}"#;

        assert_eq!(json, expected);
    }
}
