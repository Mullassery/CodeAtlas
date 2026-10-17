# Feature Inventory — CodeBoarding (upstream)

Verified from source at `main` (2026-10-17). Status legend: **C** core, **O** optional, **L** legacy/obsolete, **E** external/unsafe.

## A. CLI & workflows
| ID | Feature | Status | Notes |
|---|---|---|---|
| CLI-01 | `full` subcommand (default) — local + remote repos | C | `codeboarding_cli/commands/full_analysis.py` |
| CLI-02 | `incremental` subcommand (git-free fingerprint diff) | C | `incremental_analysis.py` |
| CLI-03 | `partial` subcommand (single component by id) | C | `partial_analysis.py` |
| CLI-04 | `codeboarding-render` (render existing analysis.json, no LLM) | C | `codeboarding_cli/render.py` |
| CLI-05 | `codeboarding-setup` (install language servers) | C | `infra/install.py` |
| CLI-06 | `--render md|html|mdx|rst` | C | 4 formats |
| CLI-07 | `--depth-cap` (safety valve, default 3) | C | persisted as `metadata.depth_cap` |
| CLI-08 | `--force` (skip cached static analysis) | C | |
| CLI-09 | `--binary-location` (custom LSP binaries) | O | VS Code extension reuse |
| CLI-10 | Remote repo clone + `--upload` to GeneratedOnBoardings | L | coupled to old output; not portable |
| CLI-11 | `--enable-monitoring` / telemetry | O | PostHog |

## B. Static analysis
| ID | Feature | Status | Notes |
|---|---|---|---|
| SA-01 | Language detection via `tokei -o json` | C | external binary |
| SA-02 | Repo ignore: `.gitignore` + `.codeboardingignore` (pathspec gitwildmatch) | C | `infra/repo_utils/ignore.py` |
| SA-03 | Python analysis (pyright) | C | |
| SA-04 | TypeScript/JavaScript analysis (typescript-language-server) | C | family batching |
| SA-05 | Go (gopls) | C | |
| SA-06 | Java (JDTLS) | C | JDK 21 |
| SA-07 | Kotlin (kotlin-lsp) | C | 360 MB, heavy |
| SA-08 | PHP (intelephense) | C | |
| SA-09 | Rust (rust-analyzer) | C | |
| SA-10 | C# (csharp-ls) | C | .NET SDK |
| SA-11 | C++ | L | enum only, no adapter |
| SA-12 | Call-graph (definitions/implementations/typeDefinition) | C | |
| SA-13 | Class hierarchies (typeHierarchy + regex fallback) | C | |
| SA-14 | Package dependencies | C | |
| SA-15 | References (SourceCodeReference) | C | |
| SA-16 | Symbol dual-alias registration + variable/function promotion | C | `symbol_table.py` |
| SA-17 | Static-analysis cache (pickle `v13`) + warm-start | C | `analysis_cache.py` |
| SA-18 | TS monorepo split (≥1000 files) | O | perf |
| SA-19 | Concurrent engine slots + memory budget | O | perf/safety |

## C. Clustering & componentization
| ID | Feature | Status | Notes |
|---|---|---|---|
| CL-01 | Deterministic name-tree partition (draft/replay) | C | `static_analyzer/clustering/names/` |
| CL-02 | Groupers: kinship (deterministic), affinity (default), planner (LLM) | C | |
| CL-03 | TreeSpec v4 (replay for incremental/partial) | C | |
| CL-04 | Hierarchical component ids (`1`, `1.2`, ...) | C | |
| CL-05 | Single-child component absorption | C | `tree_shape.py` |
| CL-06 | Expandable detection + depth-gated expansion | C | |

## D. LLM semantic enrichment
| ID | Feature | Status | Notes |
|---|---|---|---|
| LLM-01 | 14 providers (OpenAI/Anthropic/Google/Bedrock/Ollama/... ) | O | LangChain |
| LLM-02 | Component name/description/key_entities generation | C | `ScopeAnalysisAgent` |
| LLM-03 | Relation wording + evidence | C | |
| LLM-04 | Incremental scope operations (create/update/delete/noop) | C | `ScopeUpdateDecision` |
| LLM-05 | Planner grouper (JSON mode, consensus of 3 draws) | O | |
| LLM-06 | Retry/backoff/auth/quota classification | C | |
| LLM-07 | Context-window resolution (catalogs) | O | |
| LLM-08 | Read-file + method-calls tools (read-only, scoped) | C | |
| LLM-09 | Prompt profiles (STRICT for deepseek/glm/kimi/moonshot/qwen) | O | |

## E. Serialization & rendering
| ID | Feature | Status | Notes |
|---|---|---|---|
| SR-01 | `analysis.json` unified schema | C | see AUDIT §Output contract |
| SR-02 | `fingerprint.json` sidecar | C | |
| SR-03 | `file_coverage.json` | C | |
| SR-04 | Markdown + Mermaid `graph LR` | C | |
| SR-05 | HTML (Cytoscape.js + dagre) | C | |
| SR-06 | MDX | C | |
| SR-07 | reStructuredText (Sphinx mermaid) | C | |
| SR-08 | Render manifest reconciliation + overwrite protection | C | |

## F. Ecosystem (out of repo scope)
| ID | Feature | Status | Notes |
|---|---|---|---|
| EC-01 | Web platform (app.codeboarding.org) | E | not in repo |
| EC-02 | VS Code / Open VSX extension | E | |
| EC-03 | GitHub Action | E | |
| EC-04 | Health checks (`health/`) | O | `health_report.json` |
| EC-05 | Telemetry (PostHog) | O | opt-out |

## G. Excluded / unsafe / impractical to port directly
| Feature | Reason |
|---|---|
| Remote `--upload` to GeneratedOnBoardings | Coupled to a hosted artifacts repo; obsolete naming |
| Pickle-based `static_analysis.pkl` cache | Unpickle = code execution risk; Rust will use a typed format |
| C++ | Declared enum only; no adapter upstream |
| PostHog telemetry (default-on) | CodeAtlas default-off, opt-in |
| Python-only LangChain providers | Rebuilt as an optional provider facade in the wrapper |
| nodeenv/pinned tool installer | Reproduce via optional, documented setup for users who want LSP servers |