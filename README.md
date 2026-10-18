# CodeAtlas

Rust-native architecture maps for codebases. CodeAtlas scans a repository, extracts symbols and
relationships with deterministic static analysis (tree-sitter), groups code into components, and
documents the architecture as `analysis.json`, Markdown/Mermaid, HTML (Cytoscape), MDX, or
reStructuredText. An optional LLM step names and describes components.

> Rebuilding the value of [CodeBoarding](https://github.com/CodeBoarding/CodeBoarding) as an
> independent, Rust-native tool. See `NOTICE` for attribution.

## What works today

- `codeatlas full --local <repo>` produces `.codeboarding/analysis.json` + `fingerprint.json`
  (upstream-compatible schema) with deterministic, directory-derived component names.
- `codeatlas-render` (or `--render md|html|mdx|rst`) renders overview + per-component docs, with a
  manifest that reconciles stale files and refuses to overwrite unowned files.
- Optional component naming/description via an OpenAI-compatible provider (OpenAI, Ollama,
  OpenRouter, DeepSeek, LiteLLM, LM Studio). Without a provider, analysis still succeeds with
  deterministic names.
- Languages: **Python**, **Rust**, **Go**, and **JavaScript/TypeScript** (tree-sitter). Discovery recognizes
  `.java/.php/.cs`; their parsers land next.

## Quick start

```bash
pip install codeatlas
codeatlas full --local /path/to/repo
codeatlas full --local /path/to/repo --render md
codeatlas-render /path/to/repo/.codeboarding/analysis.json --format html
```

Output is written to `<repo>/.codeboarding/`. Configure an optional provider via
`OPENAI_API_KEY` (+ `OPENAI_BASE_URL`, `AGENT_MODEL`); see `codeatlas.llm`.

## Status & roadmap

The deterministic core is implemented end-to-end (discovery → tree-sitter extraction → cross-file
resolution → clustering → `analysis.json` → rendering). See `docs/FEATURE_PARITY.md` for what is
implemented vs. deferred, and `docs/IMPLEMENTATION_ROADMAP.md` for the milestone plan. Documented
limitations: tree-sitter resolution is intentionally simpler than upstream's LSP pipeline
(`docs/audit/KNOWN_GAPS.md`), clustering is a simplified deterministic directory partition, and
only Python is parsed so far.

## Layout

- `crates/core` — pure-Rust analysis engine (no Python, no network, no LLM).
- `crates/py` — PyO3 bindings (`codeatlas._core`).
- `python/codeatlas` — the Python wrapper (CLI, rendering, optional LLM).

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