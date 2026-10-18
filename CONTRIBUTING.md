# Contributing to CodeAtlas

CodeAtlas is a Rust-native analysis engine wrapped in a Python package. See
`docs/architecture/ARCHITECTURE.md` for the layout and `docs/ADRS`/`docs/architecture/ADR`
for decisions.

## Development setup

Requires Rust (stable) and Python 3.9+ (3.12 recommended), plus `maturin`.

```bash
# Rust core (no Python needed)
cargo test -p codeatlas-core

# Rust workspace (needs a Python with a shared library, e.g. `export PYO3_PYTHON=$(which python3.12)`)
cargo build --workspace && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check

# Python package (build + editable install)
maturin develop --interpreter python3.12
python -m pytest tests
```

## Conventions

- Keep the Rust core free of `pyo3`, network, and LLM dependencies.
- Deterministic output is a hard requirement: use `BTreeMap`/`BTreeSet` and stable
  sorting for any order-sensitive serialization.
- Prefer small, reviewed commits. Do not commit secrets or build artifacts
  (`dist/`, `target/`, `.codeatlas/`).
- Add tests for parsers, models, graph algorithms, and serialization; security
  paths (symlinks, oversize files, hostile input) should stay covered.
- Run `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --all --check` before opening a PR.

## Testing

- Rust unit tests: `cargo test -p codeatlas-core`.
- Python integration tests: `python -m pytest tests` (requires the built wheel).
- No test may require network access or API keys; mock LLM providers.

## License

MIT. See `LICENSE` and `NOTICE`.