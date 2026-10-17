# ADR-0005: Preserve the upstream `analysis.json` output schema

- **Status:** Accepted
- **Date:** 2026-10-17

## Context
Upstream CodeBoarding's primary artifact is `.codeboarding/analysis.json`, consumed by its web
viewer, editor extension, and documentation tooling. CodeAtlas must decide whether to preserve,
version, or replace this schema.

## Decision
Preserve the upstream `analysis.json` field layout as CodeAtlas's output format, including the
`methods_index` key convention (`"<file_path>|<qualified_name>"`), hierarchical component ids
(`"1"`, `"1.2"`), `metadata.{generated_at, source_tree_hash, repo_name, depth_level, depth_cap,
file_coverage_summary, tree_spec}`, `files`, `methods_index`, `components` (nested), and
`components_relations` (with `key_edges`/`all_edges`, `is_static`, `src_id`/`dst_id`).

## Rationale
- Maximizes compatibility with downstream readers of upstream output ("Preserve compatibility ...
  where feasible and useful" in the mission).
- Field names are functional interface facts; preserving them is low-cost and high-value.

## Consequences
- CodeAtlas declares **schema compatibility**, not byte-for-byte parity of LLM-authored text fields.
- The Rust `serde` models must match upstream Pydantic serialization rules (`exclude_none`; relations
  dumped without recursive `exclude_none`; `key_entities` with `reference_file` repo-relative).
- If upstream evolves the schema, CodeAtlas tracks released versions and documents divergence.
- The CLI/program is still named `codeatlas` (independent tool; not an imposter for `codeboarding`).