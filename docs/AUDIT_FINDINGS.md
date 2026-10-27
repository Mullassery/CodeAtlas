# Audit Report: CodeAtlas

## Overview
- **Type**: Rust-native architecture mapping tool (workspace with crates) + Python bindings
- **Languages**: Rust (workspace), Python bindings
- **Version**: 0.1.0 (workspace Cargo.toml)
- **License**: Apache-2.0
- **Purpose**: Scan repos, extract symbols/relationships via tree-sitter, cluster components, generate architecture maps (JSON/MD/Mermaid/HTML/MDX/RST), optional LLM naming

## Architecture
- **Workspace**: `crates/core` (analysis engine), `crates/py` (PyO3 bindings), `packages/` maybe, `python/` module
- **Pipeline**: discovery → tree-sitter extraction → cross-file resolution → clustering → analysis.json → rendering
- **Sidecars**: `call_graph.json` for querying
- **Languages supported**: Python, Rust, Go, JavaScript/TypeScript, Java, PHP, C#

## Entry Points
- CLI tools: `codeatlas`, `codeatlas-render` (mentioned in README)
- Python API via bindings
- Diff/comparison: `codeatlas diff`

## Dependencies
- tree-sitter ecosystem (implied), Rust workspace deps (see Cargo.lock)
- PyO3 for Python bindings (abi3 wheels)
- Optional LLM via OpenAI-compatible APIs

## Module Structure
- `crates/core/src/`: discovery, extract/{lang}.rs, clustering, pipeline, analysis, model, fingerprint
- `crates/py/src/lib.rs`: Python bindings
- Language extractors present for all listed languages

## Data/State/APIs/Contracts
- Deterministic analysis → `.codeatlas/analysis.json` + `fingerprint.json`
- Schema-compatible outputs, call graph sidecar
- Manifest reconciliation in renderer

## Security Observations
- Reads source files only (static analysis) - low risk
- Optional LLM calls to external providers if configured (API keys via env) - user responsible
- No network calls by default
- Renders HTML/Cytoscape locally

## Tests/CI
- CI badge present, tests/ directory exists
- Has docs for audits/findings, feature parity, roadmap

## Findings

| Severity | Finding | Evidence | Impact | Root Cause Hypothesis | Recommendation |
|----------|---------|----------|--------|----------------------|---------------|
| Medium | Tree-sitter resolution simpler than LSP (documented) | README/docs mention intentional simplification | May miss some cross-file refs/complex cases | Design tradeoff for determinism/speed | Document limitations clearly, consider hybrid approach |
| Low | Clustering is simplified deterministic directory partition (documented) | README states limitation | May not always match logical architecture | Simplicity over sophistication | Provide hooks for custom clustering if needed |
| Low | Optional LLM features require external API keys | README mentions env vars | Privacy/cost considerations for users | Extensibility | Ensure LLM calls are opt-in and documented |

## Strengths
- Well-structured Rust workspace
- Deterministic core without LLM dependency
- Multi-language support via tree-sitter
- Good tooling (diff, query, renderers)
- Schema stability, sidecar for call graph
- Comprehensive docs

## Limitations
- Known gaps documented (good transparency)
- Not a full LSP replacement
- Clustering heuristics simple

## Prioritized Recommendations
1. **Medium**: Keep documenting known gaps as code evolves
2. **Low**: Add tests for cross-language scenarios
3. **Low**: Consider caching strategies for large repos
