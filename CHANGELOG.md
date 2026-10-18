# Changelog

All notable changes to CodeAtlas are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Deterministic analysis core in Rust: repository discovery (gitignore +
  `.codeboardingignore`), content fingerprinting, and tree-sitter extraction
  for **Python**, **Rust**, and **Go** (symbols, qualified names, calls, imports,
  class/receiver hierarchies, cross-file unique-name call resolution).
- Deterministic directory-based component clustering with hierarchical component
  ids and per-scope (lowest-common-ancestor) relations.
- Upstream-compatible `analysis.json` output plus `fingerprint.json` and
  `file_coverage.json` sidecars.
- Python package (`codeatlas`) distributed via maturin/PyPI, with `codeatlas`
  and `codeatlas-render` CLIs.
- Renderers for Markdown + Mermaid, HTML (Cytoscape.js), MDX, and
  reStructuredText, with a render manifest that reconciles stale files and
  refuses to overwrite unowned files.
- Optional LLM component naming via OpenAI-compatible endpoints, with auth/quota
  exit codes (2/3) and deterministic-name fallback (ADR-0006).
- Security hardening: symlink and oversize-file skipping; hostile-content tests.

### Changed
- Relations are scoped to their owning (lowest-common-ancestor) component rather
  than flattened to the root scope.

### Known limitations
- Tree-sitter resolution is intentionally simpler than upstream's LSP pipeline;
  see `docs/audit/KNOWN_GAPS.md`.
- Clustering is a simplified deterministic directory partition (full
  affinity/kinship + TreeSpec replay deferred to incremental support).
- JavaScript/TypeScript, Java, PHP, C#, and Kotlin parsers not yet implemented
  (recognized in discovery, recorded as `unsupported_language` in coverage).