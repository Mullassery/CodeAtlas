# ADR-0004: Workspace layout — two Rust crates + a Python package

- **Status:** Accepted
- **Date:** 2026-10-17

## Context
The system is Rust-native but distributed as a Python/PyPI package. The recommendation to "avoid
premature micro-crates" conflicts with the need to separate the deterministic core from the
CPython binding.

## Decision
A Cargo workspace with exactly two crates in v1:

1. `codeatlas-core` — pure Rust library (models, analysis, clustering, serialization). No `pyo3`,
   no network.
2. `codeatlas-py` — `cdylib` exposing `codeatlas_rs._core`, depending only on `codeatlas-core` + `pyo3`.

A separate `python/codeatlas` package (not a Cargo crate) provides the CLI, config, rendering, and
optional LLM provider facade.

## Rationale
- Two crates is the minimum that keeps CPython ABI concerns out of the testable core while still
  allowing the whole thing to build in one `cargo` workspace command.
- Further splitting (per-language parser crates, a serialize crate) has no concrete benefit at v1
  scale and is deferred until a single crate becomes unwieldy (rule: avoid premature micro-crates).

## Consequences
- The PyO3 crate is thin and easily swapped for a different binding (e.g. a CLI binary) later.
- `cargo test --workspace` covers both crates; PyO3-specific tests run under `maturin`/`pytest`.