# ADR-0002: Use tree-sitter parsers for v1; defer LSP

- **Status:** Accepted
- **Date:** 2026-10-17

## Context
Upstream CodeBoarding extracts symbols/edges primarily through **language servers** (pyright,
typescript-language-server, gopls, JDTLS, rust-analyzer, etc.), with tree-sitter used only for call-site
introspection. LSP gives precise cross-file resolution, type hierarchy, and virtual dispatch, but
requires heavy, per-language server binaries, Node/.NET/JDK runtimes, and a JSON-RPC client.

The v1 target is a **deterministic analysis core** (owner decision). A parser strategy must be chosen
that is testable offline, deterministic, and low-dependency.

## Decision
Use **native tree-sitter grammars** for v1 symbol/edge extraction. Defer LSP adapters to a post-v1
optional layer.

## Rationale
- tree-sitter is deterministic, has no runtime binaries, and binds cleanly to Rust (`tree-sitter` +
  per-language grammars). It supports the v1 languages (Python, JS/TS, Go, Java, Rust) with mature
  grammars.
- Testability: pure libraries, no network/keys/process management.
- LSP was not chosen in v1 *despite* being what upstream uses, because its operational and dependency
  cost (tool installer, JDK/.NET/Node, memory budgets, recycler) conflicts with the v1 "minimal,
  reproducible, offline-testable" goal. LSP remains the correct tool for full type resolution later.

## Consequences
- Symbol/edge precision below LSP in v1: cross-file resolution relies on import analysis; type
  hierarchy is syntactic (`extends`/`implements`/`impl`); no virtual-dispatch expansion.
- This is a documented, deliberate divergence from upstream (reflected in `docs/audit/KNOWN_GAPS.md`
  G1). It does not silently claim LSP-level fidelity.
- The extractor layer is behind a trait (`LanguageParser`/`Extractor`) so an LSP-backed implementation
  can be added without reworking clustering or serialization.
- Kotlin/C#/C++ remain out of v1 (Kotlin/C# depend on LSP; C++ has no upstream adapter).