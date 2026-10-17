# CodeAtlas

Rust-native architecture maps for codebases. CodeAtlas scans a repository, extracts symbols and
relationships with deterministic static analysis, groups code into components, and documents the
architecture (as `analysis.json`, Mermaid/Markdown, HTML, MDX, or reStructuredText).

> Rebuilding the value of [CodeBoarding](https://github.com/CodeBoarding/CodeBoarding) as an
> independent, Rust-native tool. See the NOTICE for attribution.

## Status

Early development. The audit and migration plan are complete (`docs/audit/`); the architecture and
roadmap are in `docs/architecture/` and `docs/IMPLEMENTATION_ROADMAP.md`. The deterministic analysis
core is under construction. Not yet feature-complete: see `docs/FEATURE_PARITY.md`.

## Layout

- `crates/core` — pure-Rust analysis engine (no Python, no network, no LLM).
- `crates/py` — PyO3 bindings (`codeatlas._core`).
- `python/codeatlas` — the Python wrapper (CLI, config, rendering, optional LLM).

## Build

```bash
# Rust core (no Python needed)
cargo test -p codeatlas-core

# Rust workspace (requires a Python with a shared library on PATH for the
# binding crate, e.g. `export PYO3_PYTHON=$(which python3.12)`)
cargo build --workspace && cargo test --workspace

# Python package — build the wheel (mixed layout)
maturin build --interpreter python3.12 --out dist

# Python package — editable install for development
maturin develop --interpreter python3.12

python -c "import codeatlas; print(codeatlas.__version__)"
```

Requires Rust (stable) and Python 3.9+ (3.12 recommended), with `maturin` for the Python package.

## License

MIT. See `LICENSE` and `NOTICE`.