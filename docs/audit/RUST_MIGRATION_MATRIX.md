# Rust Migration Matrix

Maps every upstream subsystem to a CodeAtlas realization. Strategy per the confirmed plan:
**PyO3 + maturin native extension**, v1 = **deterministic analysis core** in Rust; the Python wrapper
adds CLI, config, LLM orchestration, and rendering on top.

Legend for "v1": **P1** port to Rust now, **P2** port later, **W** wrapper (Python) now, **X** exclude,
**B** blocked pending investigation.

## 1. CLI & workflows
| Upstream | CodeAtlas realization | v1 |
|---|---|---|
| `codeboarding_cli/main.py` argparse | Python `click`/`argparse` wrapper (thin), calls Rust core | W |
| `full_analysis.py` (local path) | wrapper orchestrates Rust `analyze` | W (delegates to core) |
| `incremental_analysis.py` | core fingerprint diff + partial re-analysis | P2 |
| `partial_analysis.py` (component id) | core scoped re-cluster | P2 |
| `render.py` (render-only, no LLM) | wrapper rendering from `analysis.json` | W |
| `infra/install.py` (tool installer) | optional, documented | X (v1) |
| Remote clone + `--upload` | exclude (hosted coupling) | X |

## 2. Static analysis
| Upstream | CodeAtlas realization | v1 |
|---|---|---|
| `ProjectScanner` (tokei) | **tree-sitter file discovery** (drop tokei binary dep) | P1 |
| Repo ignore (pathspec) | Rust glob/gitignore parser | P1 |
| LSP clients (`engine/lsp_client.py`, adapters) | **tree-sitter native parsers** (ADR-0002) | P1 (tree-sitter) |
| LSP-only features (typeHierarchy, virtual dispatch, monorepo split) | LSP adapters, later | P2 |
| `SymbolTable` / `CallGraph` / `Node` / `Edge` | Rust domain types (`codeatlas-core`) | P1 |
| package deps / hierarchies / references | Rust; hierarchy via tree-sitter extends/implements | P1 (partial) → P2 (full type resolution) |
| `StaticAnalysisCache` (pickle v13) | typed cache (no pickle; security) | P2 |

## 3. Clustering
| Upstream | CodeAtlas realization | v1 |
|---|---|---|
| name-tree partition (draft/replay) | Rust port of deterministic algorithm | P1 |
| Groupers affinity/kinship/planner | Rust for kinship/affinity; planner in wrapper (LLM) | P1 (affinity/kinship), W (planner) |
| `TreeSpec` v4 | serde-serialized `tree_spec` (compatible) | P1 |
| component ids `1`/`1.2` | Rust | P1 |

## 4. LLM
| Upstream | CodeAtlas realization | v1 |
|---|---|---|
| LangChain 14 providers | wrapper hits provider REST APIs (or LangChain) | W |
| `ScopeAnalysisAgent` structured output | wrapper (Pydantic) | W |
| retry/auth/quota | wrapper or shared small factory | W |
| context-window catalogs | wrapper (optional) | P2 |

## 5. Serialization & rendering
| Upstream | CodeAtlas realization | v1 |
|---|---|---|
| `analysis.json` schema | Rust `serde` structs byte-compatible | P1 |
| fingerprint/content hash (`sha256[:16]`, tree hash) | Rust `sha2` | P1 |
| file_coverage.json | Rust | P1 |
| md/mdx/html/rst + Mermaid/Cytoscape | wrapper (Jinja2 or native) | W |
| render manifest | wrapper | W |

## 6. Infra / ecosystem
| Upstream | CodeAtlas realization | v1 |
|---|---|---|
| telemetry (PostHog) | none (or later opt-in) | X |
| health checks | Rust (deterministic) | P2 |
| GitHub Action / web / VS Code | out of scope | X |
| monitoring JSON writers | none | X |

## Dependency summary (Rust core, v1)
`serde`, `serde_json`, `clap`, `thiserror`, `pyo3`, `tree-sitter` (+ python/rust/js/ts/go grammars),
`ignore`/`walkdir`, `sha2`. Python wrapper: `maturin` (build), `pydantic`, `typer`/`click`, provider SDK as needed.
Full rationale in `docs/architecture/ADR/0003-*.md` and `docs/architecture/ARCHITECTURE.md`.