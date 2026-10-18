# Changelog

All notable changes to CodeAtlas are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-10-18

### Added
- Rust import extraction: `file_coverage.json`'s `imports` map is now populated
  for Rust sources (`use` declarations, including groups, aliases, globs and
  relative paths), bringing Rust to parity with Python/Go/TypeScript.

## [0.2.0] - 2026-10-18

### Added
- `file_coverage.json` now includes a per-file `imports` map (repo-relative
  path -> sorted dotted module names), enabling structural dependency and
  repository-boundary checks that previously required text scanning.
- Release workflow (`.github/workflows/release.yml`): manylinux/musllinux,
  macOS, and Windows wheels plus sdist, published via PyPI Trusted Publishing.

## [Unreleased]

## [0.4.0] - 2026-10-18

### Added
- Deterministic analysis core in Rust: repository discovery (gitignore +
  `.codeatlasignore`), content fingerprinting, and tree-sitter extraction
  for **Python**, **Rust**, **Go**, and **JavaScript/TypeScript** (symbols,
  qualified names, calls, imports, class/receiver hierarchies, cross-file
  unique-name call resolution). `.ts`/`.tsx` are auto-disambiguated by re-parsing
  with whichever grammar produces fewer parse errors.
- Deterministic directory-based component clustering with hierarchical component
  ids and per-scope (lowest-common-ancestor) relations.
- Schema-compatible `analysis.json` output plus `fingerprint.json` and
  `file_coverage.json` sidecars.
- Python package (`codeatlas-rs`, import `codeatlas_rs`) distributed via maturin/PyPI, with `codeatlas`
  and `codeatlas-render` CLIs.
- Renderers for Markdown + Mermaid, HTML (Cytoscape.js), MDX, and
  reStructuredText, with a render manifest that reconciles stale files and
  refuses to overwrite unowned files.
- Optional LLM component naming via OpenAI-compatible endpoints, with auth/quota
  exit codes (2/3) and deterministic-name fallback (ADR-0006).
- Security hardening: symlink and oversize-file skipping; hostile-content tests.
- `codeatlas-bundle` (new `packages/codeatlas-bundle`): a pure-Python package that
  runs the analysis, renders docs, and commits them into a repository, with
  `init`/`build`/`check` commands and a `codeatlas.json` opt-in file.
- `analyze()` accepts `ignore_paths` so generated output directories can be
  excluded from discovery.

### Changed
- Relations are scoped to their owning (lowest-common-ancestor) component rather
  than flattened to the root scope.

### Known limitations
- Tree-sitter resolution is intentionally simpler than an LSP pipeline;
  see `docs/audit/KNOWN_GAPS.md`.
- Clustering is a simplified deterministic directory partition (full
  affinity/kinship + TreeSpec replay deferred to incremental support).
- Java, PHP, C#, and Kotlin parsers not yet implemented (recognized in
  discovery, recorded as `unsupported_language` in coverage).