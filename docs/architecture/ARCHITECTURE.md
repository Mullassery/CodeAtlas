# CodeAtlas Architecture

CodeAtlas is a Rust-native analysis engine wrapped in a Python package published to PyPI. This
document records the target architecture for the initial release (v1 = deterministic core + Python
wrapper). Decisions are traceable to `docs/architecture/ADR/*.md`.

## 1. Layering

```
┌─────────────────────────────────────────────────────────────┐
│ Python wrapper  (package `codeatlas`, PyPI)                 │
│   CLI (full / render), config.toml, provider facade (LLM),  │
│   rendering (md/mdx/html/rst), run orchestration            │
└──────────────────────────┬──────────────────────────────────┘
                           │ PyO3 (maturin native extension)
┌──────────────────────────▼──────────────────────────────────┐
│ codeatlas-py   (Rust cdylib → `codeatlas._core`)            │
│   Thin PyO3 surface: `analyze(path, opts) -> AnalysisJson`, │
│   version(), error mapping                                   │
└──────────────────────────┬──────────────────────────────────┘
                           │ library dep
┌──────────────────────────▼──────────────────────────────────┐
│ codeatlas-core  (pure Rust lib, no Python, no network)      │
│   discovery/ignore · tree-sitter analysis · clustering ·    │
│   domain models · serialization (analysis.json, fingerprint)│
└─────────────────────────────────────────────────────────────┘
```

- **`codeatlas-core`** is fully testable offline (no `pyo3`, no network, no API keys).
- **`codeatlas-py`** is a minimal binding layer so the core never learns CPython's ABI.
- **`codeatlas`** (Python) owns everything LLM/UX/CLI, and delegates deterministic work to the core.

## 2. Cargo workspace layout

```
crates/
  core/          # lib `codeatlas-core`
    src/
      discovery.rs    # walk + ignore + suffix→language
      model/          # Node, Edge, CallGraph, Component, Relation, Analysis...
      analysis/       # tree-sitter per-language extractors
      clustering/     # name-tree draft/replay, groupers, TreeSpec
      serialize/      # analysis.json, fingerprint, coverage
  py/            # cdylib `codeatlas._core` (PyO3)
    src/lib.rs
python/
  codeatlas/
    __init__.py   # re-export core, public API
    cli.py        # codeatlas + codeatlas-render entry points
    render/       # md/mdx/html/rst
    llm/          # provider facade (optional)
  pyproject.toml  # maturin + setuptools/pip
```

## 3. Domain model (core)

Mirrors the upstream graph concepts so `analysis.json` stays compatible:

- `Node { fully_qualified_name, file_path, line_start, line_end, col_start, type: NodeType }`
- `Edge { source, destination, call_sites: Vec<CallSite> }` (`CallSite { file, line, column }`)
- `ReferenceEdge { src, dst, kind (CONTAINS|INHERITS|TYPEREF|IMPORT) }`
- `CallGraph { nodes, edges, reference_edges }`
- `MethodIndexEntry`, `FileEntry`, `Component`, `Relation`, `RelationEdge`, `AnalysisInsights`,
  `AnalysisMetadata`, component-id assignment (`1`, `1.2`, ...).
- Canonical method key `"{file_path}|{qualified_name}"` used by `methods_index` and relation edges.

## 4. Analysis pipeline (core)

1. **Discover** source files (suffix → language), applying gitwildmatch ignore rules.
2. **Parse** each file with the language's tree-sitter grammar; collect symbols + spans + imports.
3. **Resolve** intra-project symbols (qualified names, imports) and build call edges + hierarchies
   + package/module dependencies. This is a deliberately simpler resolution than upstream's LSP
   pipeline (ADR-0002).
4. **Cluster** deterministically (name-tree partition; affinity/kinship groupers) into a nested
   component tree with a replayable `TreeSpec`.
5. **Serialize** `analysis.json`, `fingerprint.json`, `file_coverage.json`.

Deterministic output is a hard requirement; everything above is pure and ordering-stable.

## 5. Python wrapper responsibilities

- Parse CLI args; load `~/.codeatlas/config.toml` (or env); resolve providers.
- Call the core (`analyze`) and read the returned model.
- **Optional semantic enrichment:** if a provider is configured, name/describe components and
  relations and set `key_entities`; otherwise fall back to deterministic names (ADR-0006).
- Render markdown/html/mdx/rst and maintain the render manifest.
- Map Rust errors to exit codes (0/1/2/3).

## 6. Cross-cutting concerns

- **Error handling:** core uses `thiserror` typed errors; `codeatlas-py` maps them onto a small set
  of `PyErr`s with stable message/kind; wrapper maps kind → exit code.
- **Determinism:** stable sorting everywhere (`(start_line, end_line, qualified_name)`, dotted-id
  sort); `BTreeMap`/`BTreeSet` for ordered iteration.
- **Security:** analyzed content is only read/parsed; symlinks are followed but outputs are written
  to a dedicated output dir; `analysis.json`/fingerprint writes are atomic (temp + rename); no
  secrets are logged.
- **Resources:** file-size and total-bytes caps; parsing is streaming where possible (later).

## 7. Build & packaging

- `maturin build --release --locked` builds the extension; `--interpreter python3.12` for CI.
- `[tool.maturin] module-name = "codeatlas._core"`, `manifest-path = "crates/py/Cargo.toml"`.
- CI (GitHub Actions) runs `cargo fmt --check`, `cargo clippy`, `cargo test`, `maturin build`, and
  `pytest` on the wrapper, on Linux + macOS; manylinux wheels for release.

## 8. Rationale for NOT a micro-crate workspace

Only two Rust crates (core + py bindings) in v1. Further splitting (per-language parsers, separate
serialize crate) is deferred until a concrete size/complexity justification exists (rule: avoid
premature micro-crates). See ADR-0004.