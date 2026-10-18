# Implementation Roadmap

Versioned, incremental plan. Each milestone is small, has acceptance criteria, and keeps the build
green. Status is tracked in `docs/PROJECT_BACKLOG.md` during execution.

## M0 — Project skeleton (Phase 3)
- Monorepo layout: Cargo workspace (`codeatlas-core` + `codeatlas-python`), Python package.
- `maturin` build producing a `codeatlas_rs._core` extension with a trivial `version()` function.
- CI (format, clippy, `cargo test`, `maturin build`, `pytest`).
- *Acceptance:* `cargo build` and `maturin develop` succeed; extension imports.

## M1 — Domain models & serialization (R4)
- Rust structs for `Node`, `CallGraph`, `MethodEntry`, `Component`, `Relation`, `AnalysisInsights`,
  `AnalysisMetadata`, component-id assignment.
- serde serialization of `analysis.json` (upstream-compatible fields).
- *Acceptance:* round-trip/golden tests; a hand-built graph serializes to valid schema.

## M2 — Discovery + ignore + fingerprint (R1)
- Suffix → language map, file walker, gitwildmatch ignore, `.codeatlasignore`; `sha2` per-file
  hashes + tree hash; `fingerprint.json`, `file_coverage.json` stubs.
- *Acceptance:* discovery/ignore fixtures; deterministic hash tests.

## M3 — Tree-sitter analysis for one language: Python (R2)
- `python` grammar: symbols, qualified names, calls, imports, `class X(Base)`, spans, call sites.
- *Acceptance:* golden graph on a Python fixture; unit tests for each extractor.

## M4 — Clustering (R3)
- Port name-tree draft/replay + affinity/kinship groupers + TreeSpec.
- *Acceptance:* partition equals reference on fixtures; replay == draft.

## M5 — End-to-end vertical slice (R5)
- Python wrapper: `codeatlas --local <repo>` → full pipeline → `.codeatlas/analysis.json`
  (+ fingerprint + coverage); deterministic names when no provider.
- *Acceptance:* run on a committed fixture repo; asserts on output.

## M6 — Rendering (R7)
- md/mdx/html/rst generators + mermaid/cytoscape; render manifest.
- *Acceptance:* golden rendered docs on a fixture.

## M7 — Semantic enrichment (R6)
- Wrapper provider facade; optional LLM naming/description; auth/quota exit codes 2/3.
- *Acceptance:* mock-provider tests; no-provider path still succeeds.

## M8 — Hardening (Phases 5–6)
- Error paths, symlink/path-traversal guards, size limits, security tests, benchmarks.
- *Acceptance:* `cargo test` + `pytest` green; clippy clean; no network in core tests.

## M9 — Later languages & parity (post-v1)
- Add Rust, JS/TS, Go, Java grammars; then LSP adapters; incremental/partial; health; packaging.

## Milestone ↔ phase traceability
| Milestone | Phase |
|---|---|
| M0 | 3 |
| M1–M4 | 4 |
| M5–M7 | 4 |
| M8 | 5–6 |
| M9 | 4 (continued) |
| docs/CI/release | 7–8 |