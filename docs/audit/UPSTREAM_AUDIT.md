# Upstream Forensic Audit — CodeBoarding → CodeAtlas

- **Upstream:** https://github.com/CodeBoarding/CodeBoarding (Org: `CodeBoarding`)
- **Audited revision:** `main` at 2026-10-17 (shallow clone)
- **License:** MIT (`Copyright (c) 2025 CodeBoarding`)
- **Language:** Python (requires `>=3.12,<3.13`; `.python_version` = `3.13` in dev; README says 3.12)
- **Repository size:** ~108 MB on disk; 128 test files; ~30k LOC source + ~39k LOC tests
- **Distribution:** PyPI package `codeboarding` (entry points `codeboarding`, `codeboarding-setup`, `codeboarding-render`)
- **Maturity:** Beta (`Development Status :: 4 - Beta`), v0.14.2 latest tag, 50 open issues, ~2.6k stars, 213 forks

This document records **verified facts** from source inspection. Assumptions are labelled `[ASSUMPTION]`.

---

## 1. What the product does

CodeBoarding turns a codebase into an interactive architecture map. The repository here is the
**open-source analysis engine + CLI**. The interactive "web platform" (app.codeboarding.org), the
VS Code extension, and the GitHub Action are **external and out of scope** for this repo.

The core loop, verified end-to-end from `codeboarding_cli/main.py` → `codeboarding_workflows/analysis.py`
→ `diagram_analysis/diagram_generator.py`:

1. **Scan** the repository to detect supported languages (via the external `tokei -o json` binary).
2. **Static analysis** (`static_analyzer/`): launch per-language LSP servers, extract symbols,
   call-graph edges, class hierarchies, package dependencies, and references.
3. **Cluster** (`static_analyzer/clustering/`): deterministically partition code into a nested
   component tree (no ML).
4. **Semantic enrichment** (`agents/` + LLM): name/describe components and relations using a
   configured LLM provider.
5. **Serialize** to `.codeboarding/analysis.json` plus sidecar artifacts, and optionally render
   Markdown/Mermaid, HTML (Cytoscape), MDX, or reStructuredText documentation.

### Output contract (`.codeboarding/`)
| Artifact | Purpose |
|---|---|
| `analysis.json` | The unified, versioned architecture graph. Primary deliverable. |
| `fingerprint.json` | Whole-tree `{path: truncated sha256}` map; enables git-free incremental diff. |
| `file_coverage.json` | Analyzed vs excluded file accounting. |
| `static_analysis.pkl` / `.sha` | Pickled static-analysis cache (version tag `v13`). |
| `codeboarding_version.json` | Engine version stamp. |
| `overview.md` / `on_boarding.md` + `<component>.md` | Rendered docs (optional). |
| `health/health_report.json` | Health-check output. |

---

## 2. CLI & command surface

Entry point: `codeboarding_cli/main.py::main`. `full` is the **default** subcommand (injected
when the first arg is not `full`/`incremental`/`partial`).

```
codeboarding [full|incremental|partial] [--local PATH] [repo_url ...]
             [--output-dir PATH] [--project-name NAME] [--render md|html|mdx|rst]
             [--binary-location PATH] [--enable-monitoring]
             [--depth-cap N]            # full only (default 3)
             [--force]                  # full only
             [--upload]                 # full remote only
             [--component-id "1.2"]     # partial only
codeboarding-render ANALYSIS.json [--format ...] [--output-dir ...]
codeboarding-setup                      # install language servers
```

Remote-repo mode clones via GitPython into `./repos/<name>` and writes Markdown/JSON into
`cwd/.codeboarding/`. Remote + `--upload` pushes to `GeneratedOnBoardings` (now `awesome-architecture-mds`).

Exit codes (from `agents/llm_errors.py`): `0` ok, `1` generic failure, `2` auth failure,
`3` quota exhausted.

## 3. Configuration

- `~/.codeboarding/config.toml` (TOML), created on first run. Sections: `[provider]` (14 providers),
  `[llm]` (`agent_model`, `context_window`), `[clustering]` (`grouper`).
- Environment variables take precedence over the file. Selection env vars for providers:
  `OPENAI_API_KEY`, `OPENAI_BASE_URL`, `ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`, `GOOGLE_API_KEY`,
  `VERCEL_API_KEY`, `AWS_BEARER_TOKEN_BEDROCK`, `OLLAMA_BASE_URL`/`OLLAMA_HOST`, `DEEPSEEK_API_KEY`,
  `GLM_API_KEY`, `KIMI_API_KEY`, `OPENROUTER_API_KEY`, `ORCAROUTER_API_KEY`, `REQUESTY_API_KEY`,
  `LITELLM_BASE_URL`(+`LITELLM_API_KEY`), `CEREBRAS_API_KEY`.
- `AGENT_MODEL` overrides the provider default model.
- Static-analysis tuning: `CODEBOARDING_LSP_REQUEST_TIMEOUT`, `CODEBOARDING_MAX_CONCURRENT_ENGINES`,
  `CODEBOARDING_LSP_MEMORY_BUDGET_MB`.
- Grouper selection: `CODEBOARDING_GROUPER` = `affinity` (default) | `kinship` | `planner`.
- Telemetry opt-out: `DO_NOT_TRACK` or `CODEBOARDING_TELEMETRY=false`.

---

## 4. Static analysis subsystem (`static_analyzer/`)

### Languages & LSP servers (verified from `engine/adapters/` + tool registry)
| Language | Suffixes | LSP server | Notes |
|---|---|---|---|
| Python | `.py` | `pyright-langserver --stdio` | |
| TypeScript/JavaScript | `.ts .tsx .mts .cts .js .jsx .mjs .cjs` | `typescript-language-server --stdio` (+ tsserver plugin) | one bucket for the family (`FAMILY_OWNER[JS]=TS`) |
| Go | `.go` | `gopls serve` | needs `go` on PATH |
| Java | `.java` | JDTLS (`java -jar org.eclipse.equinox.launcher_*.jar`) | JDK 21; heap by file count |
| Kotlin | `.kt` (not `.kts`) | kotlin-lsp `intellij-server` | ~360 MB, JetBrains Free Plugin License |
| PHP | `.php` | `intelephense --stdio` | |
| Rust | `.rs` | `rust-analyzer` | needs cargo |
| C# | `.cs` | `csharp-ls` | needs .NET SDK; `dotnet restore` prepare |
| C++ | `.cpp .cc .cxx .hpp .hh .hxx .h` | *(no adapter — enum only)* | declared but not implemented |

`NodeType` = LSP `SymbolKind` ints 1–26 (FILE..TYPE_PARAMETER). `Language`/`SourceSuffix` enums in
`static_analyzer/config.py`.

### Pipeline (`StaticAnalyzer.analyze` in `static_analyzer/__init__.py`)
1. `ProjectScanner` runs `tokei -o json` (tree-sitter-free; the `tree-sitter-*` PyPI deps are for
   the **engine's own** `SourceInspector`/`potential_calls` call-site detection + Kotlin/C# grammar
   introspection, not the primary symbol source).
2. Per-language `EngineConfig` (TypeScript monorepos split at ≥1000 files; Java/C# per-project).
3. `CallGraphBuilder.build`: discover symbols (`documentSymbol`) → build edges (`textDocument/definition`,
   `implementation`, `typeDefinition`) → hierarchies (`typeHierarchy` + regex fallback) → package deps.
4. `ResultConverter.convert_to_codeboarding_format` → `CallGraph` (nodes + edges + reference edges
   CONTAINS/INHERITS) + references + `{language: {...}}` dicts.
5. Persist/incremental via `StaticAnalysisCache` (pickle, tag `v13`, filelock, warm-start).

The LSP client is a synchronous JSON-RPC-over-stdio implementation (`static_analyzer/engine/lsp_client.py`):
`initialize`/`initialized` handshake, daemon reader thread, batched requests (50/round), retries on
transient content-modified, robustness notifications for tsserver/rust-analyzer/intelephense/kotlin-lsp.

### Clustering (`static_analyzer/clustering/names/`)
**Deterministic name-tree partition** (NOT Infomap; networkX is only used for graph export and health
cycle detection). A `Trie` of directory positions is walked via a "ladder" of splitting rules
(UNMERGE → FRONTIER/SEGMENT → LAYERS → FILES → ROLE → ISLAND), grouping units with a `Grouper`
(`kinship` deterministic or `affinity` aka kinship + call-graph fold toward ≤9 components, or LLM
`planner`). The result is a `TreeSpec` (version 4) of `ComponentRule`s (prefix/term matching) that can
be **replayed** deterministically for incremental/partial runs. Component IDs are hierarchical dotted
strings (`"1"`, `"1.2"`, `"1.2.1"`); `source_cluster_ids` are qualified cluster ids.

---

## 5. LLM subsystem (`agents/`)

- Built on **LangChain** chat models + **LangGraph** `create_agent` + **Pydantic v2** structured output.
- **14 providers** via LangChain SDKs: OpenAI, Vercel, Anthropic, Google, AWS Bedrock, Cerebras,
  Ollama, DeepSeek, GLM, Kimi, OpenRouter, OrcaRouter, Requesty, LiteLLM.
- Two LLM roles: `TreePlannerAgent` (optional planner grouper, JSON mode, 3 concurrent draws +
  consensus) and `ScopeAnalysisAgent` (per-scope naming/description/key_entities + relations, via a
  structured-output tool `ScopeAnalysisResult`).
- **Determinism without LLM** is possible for grouping/membership/static relations; only semantic
  naming/description and non-static relation wording require the model. With no provider configured,
  the full pipeline aborts (exit 1/2) rather than shipping folder-named components.
- Retry: 5 attempts, full-jitter exponential backoff 4s→60s, honors `Retry-After`; SDK retries disabled.
- Context windows resolved from env → config → Ollama `/api/show` → models.dev/litellm/OpenRouter
  catalogs (24h disk cache) → fallback 256k/64k.
- Tools exposed to the LLM (read-only): `readFile` (300-line window, repo-contained) and
  `getMethodCalls` (CFG edges).
- **No prompt-injection guards, no sandbox, no code execution of analyzed repos.** Repository text is
  only read.

## 6. Diagram & output generation

`DiagramGenerator` (90 KB orchestrator) stages: deterministic analysis → scope assembly →
per-scope enrichment → threaded frontier expansion (depth-capped) → `finalize_and_save`
(strip ignored, assert containment, rebuild global relations, absorb single-child components,
distinguish name collisions) → `analysis.json` + sidecars.

Rendering (`output_generators/`): `md` (Mermaid `graph LR`), `mdx` (Mermaid + frontmatter),
`html` (Cytoscape.js + dagre), `rst` (Sphinx `mermaid` directive). Root doc `overview`/`on_boarding`,
per-expanded-component `<sanitized name>.<ext>`. A manifest (`.codeboarding-render.json`) reconciles
and unlinks stale rendered files and **refuses to overwrite unowned files**.

## 7. Health, telemetry, infra

- `health/` plugin framework: function-size, circular-dependency (networkX cycles), unused-code
  (LSP diagnostics) checks → `health_report.json`.
- Telemetry (PostHog, on by default, opt-out): `analysis_started/analysis_completed`, `repo_scanned`,
  `lsp_analysis_result`, `$exception`, MCP events. Explicitly never collects source, filenames,
  repo names, prompts, outputs, keys.
- Tool installer (`codeboarding-setup`) downloads pinned language servers into `~/.codeboarding/servers/`
  (tokei, gopls, pyright, typescript-language-server, intelephense, csharp-ls, JDTLS, kotlin-lsp,
  rust-analyzer, embedded Node 20.18.1). SHA256 pins exist for some; rust-analyzer/JDTLS are
  tag/build-pinned only.
- CI (6 workflows): tool build, self-analysis sync, PR review action, integration tests (dispatched),
  pre-commit + mypy + `pytest --cov --cov-fail-under=70`, evals update.

## 8. Security & reliability observations (inputs to CodeAtlas)

- Analyzed repo content is read-only (no execution), but there is **no isolation** beyond that and
  **no prompt-injection defense**; comments/config/prompts must be treated as untrusted.
- Atomic writes (temp + `os.replace`) with file locks for the main artifacts; render manifest prevents
  clobbering unrelated files.
- LSP requests bound by timeouts; memory budgeted across concurrent engines; servers recycled/restarted.

## 9. Verified vs assumed

- **Verified:** CLI commands/flags, provider list, LSP servers per language, `analysis.json` schema,
  component-id scheme, clustering is deterministic (not Infomap), MIT license, Python 3.12-only,
  default-grouper=affinity, full pipeline requires an LLM, exit codes 2/3.
- **Assumed (not exercised):** exact upstream *output bytes* of `analysis.json` for a given repo were
  not diffed against a live run [ASSUMPTION]; the `tokei` JSON contract is consumed but not re-produced.