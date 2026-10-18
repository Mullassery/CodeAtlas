# CodeAtlas

Rust-native architecture maps for codebases. CodeAtlas scans a repository, extracts symbols and
relationships with deterministic static analysis (tree-sitter), groups code into components, and
documents the architecture as `analysis.json`, Markdown/Mermaid, HTML (Cytoscape), MDX, or
reStructuredText. An optional LLM step names and describes components.

## What works today

- `codeatlas full --local <repo>` produces `.codeatlas/analysis.json` + `fingerprint.json`
  (schema-compatible output) with deterministic, directory-derived component names.
- `codeatlas-render` (or `--render md|html|mdx|rst`) renders overview + per-component docs, with a
  manifest that reconciles stale files and refuses to overwrite unowned files.
- Optional component naming/description via an OpenAI-compatible provider (OpenAI, Ollama,
  OpenRouter, DeepSeek, LiteLLM, LM Studio). Without a provider, analysis still succeeds with
  deterministic names.
- Languages: **Python**, **Rust**, **Go**, **JavaScript/TypeScript**, **Java**,
  **PHP**, and **C#** (tree-sitter).

## Quick start

```bash
pip install codeatlas-rs
codeatlas full --local /path/to/repo
codeatlas full --local /path/to/repo --render md
codeatlas-render /path/to/repo/.codeatlas/analysis.json --format html
codeatlas diff old/.codeatlas/analysis.json new/.codeatlas/analysis.json --fail-on-change
```

Output is written to `<repo>/.codeatlas/`. Configure an optional provider via
`OPENAI_API_KEY` (+ `OPENAI_BASE_URL`, `AGENT_MODEL`); see `codeatlas_rs.llm`.

## PR architecture diff

`codeatlas diff` compares two architecture maps, and
`.github/actions/architecture-diff` runs it as a PR check. See
[`docs/PR_DIFF.md`](docs/PR_DIFF.md).

## Audit findings

The 2026-10-19 implementation audit — including the Java extractor work,
verification results, and remaining gaps — is recorded in
[`AUDIT_FINDINGS.md`](AUDIT_FINDINGS.md).

## Status & roadmap

The deterministic core is implemented end-to-end (discovery → tree-sitter extraction → cross-file
resolution → clustering → `analysis.json` → rendering). See `docs/FEATURE_PARITY.md` for what is
implemented vs. deferred, and `docs/IMPLEMENTATION_ROADMAP.md` for the milestone plan. Documented
limitations: tree-sitter resolution is intentionally simpler than an LSP pipeline
(`docs/audit/KNOWN_GAPS.md`) and clustering is a simplified deterministic directory partition.

## Bundle into any repository

`codeatlas-bundle` versions the architecture map inside a repository: it runs the analysis, renders
the docs, and (optionally) commits them so the map evolves with the code. See
[`packages/codeatlas-bundle`](packages/codeatlas-bundle).

```bash
pip install codeatlas-rs codeatlas-bundle
cd my-repo && codeatlas-bundle init && codeatlas-bundle build --commit
```

## Layout

- `crates/core` — pure-Rust analysis engine (no Python, no network, no LLM).
- `crates/py` — PyO3 bindings (`codeatlas_rs._core`).
- `python/codeatlas_rs` — the Python wrapper (CLI, rendering, optional LLM).
- `packages/codeatlas-bundle` — pure-Python package that bundles the docs into other repositories.

## Development

```bash
# Rust core (no Python needed)
cargo test -p codeatlas-core

# Rust workspace (needs a Python with a shared library, e.g. `export PYO3_PYTHON=$(which python3.12)`)
cargo build --workspace && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# Python package
maturin build --release --interpreter python3.12 --out dist
maturin develop --interpreter python3.12
python -m pytest tests
```

Requires Rust (stable) and Python 3.9+ (3.12 recommended), with `maturin` for the Python package.

## License

MIT. See `LICENSE` and `NOTICE`.