# Known Gaps

Verified gaps between upstream behavior and what CodeAtlas can reproduce, plus areas requiring
further investigation. These drive `docs/FEATURE_PARITY.md` and the backlog.

## G1 — LSP-grounded precision vs tree-sitter approximations
Upstream symbol/edge extraction is **LSP-first** (definition/implementation/typeDefinition/typeHierarchy).
CodeAtlas v1 uses **tree-sitter** directly. This changes:
- Cross-file symbol resolution precision (tree-sitter needs its own import/scope resolution).
- Virtual dispatch, method groups, C# iterators/collection-initializers, Kotlin constructors.
- Class hierarchy is partial without LSP typeHierarchy.
**Mitigation:** v1 targets Python/Rust/Go/Java/TS/JS symbol + intra-project call edges (imports +
direct calls) and class hierarchies via `extends`/`implements`/`impl` syntax. LSP adapters are a P2
layer for full type resolution.

## G2 — Full pipeline requires an LLM upstream
Upstream aborts (rather than shipping folder-named components) when no provider is configured.
CodeAtlas v1 ships the deterministic core **without** requiring an LLM; semantic naming/description
is produced by the wrapper only when a provider is configured. This widens usability but diverges
from upstream's "fail rather than mislabel" policy — documented as a deliberate, verifiable difference.

## G3 — No committed golden `analysis.json` diff against a live upstream run
The upstream `.codeboarding/analysis.json` (1.8 MB) shows the real schema, but we have not run a
byte-level comparison of CodeAtlas output against a fresh upstream run. `[ASSUMPTION]` until a
fixture repo is analyzed by both tools.

## G4 — Tool/language-server installer deferred
`codeboarding-setup` (nodeenv, pinned Node 20, JDTLS, kotlin-lsp, csharp-ls, etc.) is out of v1 scope.
Users needing LSP-based analysis must install servers themselves (documented later). Kotlin defered
beyond v1 (large, non-standard license).

## G5 — C++ declared but unimplemented upstream
Upstream has `.cpp/.cc/...` suffixes and a `Language.CPP` enum but no adapter. Not a parity gap:
CodeAtlas also defers C++.

## G6 — Telemetry & health deferred
PostHog (default-on) is excluded from CodeAtlas (privacy). Health checks are deferred to P2 (they
depend on graph + class hierarchies, which are P1/P2).

## G7 — Python-API + CLI surface not yet frozen
Upstream exposes `run_full`, `build_generator`, `DiagramGenerator(depth_cap=...)` and a `depth_cap`
vs `depth_level` metadata distinction. CodeAtlas will mirror the CLI flags (`--local`, `--depth-cap`,
`--render`, `--force`, `--project-name`, `--output-dir`) and core Python functions, but exact
signatures are finalized in Phase 3/4.

## G8 — Rust-side context-window / retry parity is wrapper-owned
Provider-specific retries, auth classification, and context-window resolution are implemented in the
Python wrapper (or a later Rust facade) for v1; the core never blocks on network.

## G9 — Reproducibility of component naming
Even upstream's deterministic grouper can yield LLM-dependent names; the *membership* (component
tree + file/method assignment) is deterministic, but `name`/`description`/`relation` wording are not.
CodeAtlas conveys the same split and will add golden tests only for the deterministic surface.

## G10 — Blocked: exact upstream test-fixture access
The upstream unit tests are in-repo (128 files), but the **integration suite lives in the private
`CodeBoarding/CodeBoarding-tests` repo** and is not accessible here. CodeAtlas builds its own fixtures
and cannot reproduce upstream's private integration assertions.