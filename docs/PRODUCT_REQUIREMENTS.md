# CodeAtlas — Product Requirements

CodeAtlas is an independent, Rust-native codebase architecture analysis tool with a Python/PyPI
wrapper. It rebuilds the valuable behavior of CodeBoarding: discover code structure, identify
components and relationships, analyze dependencies, and generate architecture documentation.

## 1. Product scope (v1)

### 1.1 Personas & goals
- **Developer onboarding** — answer "what is this repo, what are its parts, how do they connect?"
- **Architecture documentation** — generate reproducible, versionable architecture docs/diagrams.
- **PR/change review** (later) — diff architecture between baselines.

### 1.2 Non-goals (v1)
- LSP-backed full type resolution (post-v1 optional layer).
- Remote repo cloning + hosted upload.
- Telemetry (privacy: none by default).
- Interactive web UI / VS Code extension / GitHub Action.
- `codeboarding-setup` binary installer.

## 2. Requirements (functional)

Each requirement is tagged `[core delivered by Rust]` / `[wrapper]` and carries acceptance criteria.

### R1 — Repository discovery & ignore [core]
- R1.1 Walk a repo root, collect source files by suffix.
- R1.2 Honor `.gitignore` and `.codeboarding/.codeboardingignore` (gitwildmatch semantics).
- R1.3 Always ignore `.git`, `.codeboarding`, `node_modules`, vendored/build dirs.
- R1.4 Detect languages from suffixes (python, js, ts, jsx, tsx, go, java, rust, php, csharp, ...).
- *Acceptance:* given a fixture with mixed files + ignores, the discovered/ignored sets match spec.

### R2 — Tree-sitter static analysis [core]
- R2.1 Parse source and extract symbols (functions, classes, methods, fields) with qualified names
  and source spans.
- R2.2 Extracall edges (intra-file direct calls; cross-file via imports/basic resolution) with call sites.
- R2.3 Extract class hierarchies (`extends`/`implements`/`impl`).
- R2.4 Extract package/module dependencies.
- R2.5 Deterministic output for a fixed input.
- *Acceptance:* unit tests per language; golden graphs for fixtures.

### R3 — Component clustering [core]
- R3.1 Deterministic name-tree partition equivalent to upstream `affinity`/`kinship` groupers.
- R3.2 Hierarchical component IDs (`1`, `1.2`, `1.2.1`) and `source_cluster_ids`.
- R3.3 Replayable `TreeSpec` (for later incremental/partial).
- *Acceptance:* cluster membership equals reference partition on fixtures.

### R4 — `analysis.json` serialization [core]
- R4.1 Emit the upstream `analysis.json` schema (metadata, description, files, methods_index,
  components, components_relations) byte-parseable by upstream readers.
- R4.2 Emit `fingerprint.json` (per-file `sha256[:16]`) and whole-tree hash.
- R4.3 Emit `file_coverage.json`.
- *Acceptance:* golden-file comparisons; schema validation; upstream parser compatibility (where testable).

### R5 — Python wrapper + CLI [wrapper]
- R5.1 PyPI package `codeatlas` with a `codeatlas` CLI mirroring `--local`, `--output-dir`,
  `--project-name`, `--depth-cap`, `--render md|html|mdx|rst`, `--force`.
- R5.2 `codeatlas-render` renders an existing `analysis.json` without analysis.
- R5.3 Python API `run_full(...)` / `build_generator(...)` / `DiagramGenerator(...)` mirroring upstream names.
- *Acceptance:* `pip install` + `codeatlas --local ./fixture` produces `.codeboarding/analysis.json`.

### R6 — Semantically named components [wrapper, optional LLM]
- R6.1 When a provider is configured, name/describe components and relations (2–5 key entities each).
- R6.2 Without a provider, produce deterministic folder-derived names (no crash).
- *Acceptance:* with a stubbed/mock provider, names flow into analysis.json; without, run succeeds
  with deterministic names.

### R7 — Rendering [wrapper]
- R7.1 Markdown + Mermaid `graph LR`, HTML (Cytoscape), MDX, RST.
- R7.2 Overview doc + per-expanded-component docs; render manifest prevents clobbering unowned files.

### R8 — Errors, exit codes, resource limits [core + wrapper]
- R8.1 Structured errors; exit codes `0/1/2/3` compatible with upstream (auth=2, quota=3).
- R8.2 No execution of analyzed repo code; bounded file sizes/URLs; atomic writes.

## 3. Non-functional requirements

- **Determinism:** same input → byte-identical output for the deterministic surface.
- **Offline-testable:** core unit/integration tests need no network or API keys.
- **Minimal deps:** Rust core uses few, audited crates; Python wrapper uses pydantic + a CLI lib.
- **Reproducible builds:** `maturin build --release --locked` + manylinux/CI matrix.
- **Security:** untrusted repo content never executed; outputs never overwrite unrelated files;
  keys never logged.

## 4. Out of scope for v1 (tracked for later)
Incremental/partial analysis, LSP adapters, Kotlin/C#/Java full type resolution, health checks,
GitHub Action, planner LLM grouper, context-window catalogs. See `docs/FEATURE_PARITY.md`.