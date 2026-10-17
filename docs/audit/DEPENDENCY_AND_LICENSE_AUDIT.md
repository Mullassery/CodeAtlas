# Dependency & License Audit — CodeBoarding → CodeAtlas

## 1. Upstream license

- **MIT**, `Copyright (c) 2025 CodeBoarding`. Verified from `LICENSE`.
- MIT permits use, modification, sublicensing, and redistribution **with retention of the copyright
  and permission notices** in all copies/substantial portions.
- **Obligation for CodeAtlas:** this is a clean-room *rebuild in Rust* informed by the upstream design
  (interfaces, data models, behavior), not a verbatim copy. No upstream Python/comment/prose is
  copied. Where a data-model *shape* is preserved for output compatibility (e.g. `analysis.json`
  field names), field names are functional interface facts, not copyrightable expression; MIT would
  permit reuse in any case. CodeAtlas ships its own MIT license with attribution note in `NOTICE`.

## 2. Upstream runtime dependency profile (Python)

Key hard dependencies (from `pyproject.toml`) and their licenses, for reference only (CodeAtlas does
not vendor these):

| Dependency | Purpose | License (nominal) |
|---|---|---|
| langchain* (8 packages), langgraph | LLM orchestration | MIT |
| pydantic | models/schema | MIT |
| networkx | graph export + cycle detection | BSD-3-Clause |
| tree-sitter(+language grammars) | engine call-site introspection | MIT (runtime) |
| pathspec | gitignore matching | MPL-2.0 |
| posthog | telemetry | MIT |
| fastapi, uvicorn, docker, gitpython, dulwich, requests, etc. | app/CI/support | mixed OSI |
| tqdm, rich, pyyaml, jinja2, markdown*, pydantic | UX/rendering | mixed OSI |

## 3. Language-server / tool licenses (downloaded at install time)

| Tool | License | SHA256 pinned? |
|---|---|---|
| tokei | MIT (Rust) | yes (3 platforms) |
| gopls | BSD-3-Clause | yes |
| pyright (npm) | MIT | npm version pin |
| typescript-language-server + typescript | Apache-2.0 + Apache-2.0 | version pin |
| intelephense | MIT (free core) | version pin |
| csharp-ls | MIT (dotnet tool) | version pin |
| JDTLS (Eclipse) | EPL-2.0 | tag/build only |
| kotlin-lsp | **JetBrains Free Plugin License** | yes (6 assets) |
| rust-analyzer | MIT/Apache-2.0 | tag only |

**CodeAtlas consequence:** the download tool installer is out of scope for v1. Any future installer
must document the Kotlin (JetBrains Free Plugin License) and JDTLS (EPL-2.0) obligations and keep SHA256
verification.

## 4. Decisions for CodeAtlas

1. **Rust core dependencies must be minimal and audited.** Proposed core set (see `docs/architecture`
   ADR-0003): `serde`/`serde_json` (MIT/Apache-2.0), `clap` (MIT/Apache-2.0), `thiserror` (MIT/Apache-2.0),
   `pyo3` (MIT/Apache-2.0), `tree-sitter` + grammars (MIT), `ignore`-equivalent (custom pathspec or
   `globset` MIT/Unlicense), `ignore`/`walkdir` (Unlicense/MIT). Hash via `sha2` (MIT/Apache-2.0).
2. **No PostHog, no docker, no nodeenv** in v1.
3. **LSP servers are an optional runtime dependency**, not vendored. The deterministic v1 core uses
   tree-sitter directly for symbol/edge extraction; LSP adapters are a later, documented, opt-in layer.
4. **Attribution:** README `NOTICE`/`LICENSE` will credit CodeBoarding (MIT) and note design provenance.

## 5. Open items / risks

- **Rust-analyzer and JDTLS lack content hashes** in the upstream registry; replicate only with pins.
- **kotlin-lsp** is large and under a non-standard license; defer Kotlin to post-v1.
- Exact per-grammar tree-sitter licenses must be enumerated when grammars are added (filtered by
  `license = "MIT"`/permissive only in `Cargo.toml`).
- Clean-room authorship: no upstream source strings should appear in CodeAtlas output except
  functional identifiers mandated by `analysis.json` compatibility.