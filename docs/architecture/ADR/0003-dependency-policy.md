# ADR-0003: Dependency policy

- **Status:** Accepted
- **Date:** 2026-10-17

## Context
The mission mandates "minimal dependencies", "reproducible builds", and justifying major
dependencies. The Rust core must be auditable and license-clean.

## Decision
The Rust **core** crate depends only on:

| Crate | Purpose | License |
|---|---|---|
| `serde` + `serde_json` | (de)serialization of `analysis.json` etc. | MIT/Apache-2.0 |
| `thiserror` | typed errors | MIT/Apache-2.0 |
| `tree-sitter` + grammars (python, rust, javascript, typescript, go, java) | parsing | MIT |
| `ignore` (or `globset` + `walkdir`) | gitwildmatch ignore + walking | Unlicense/MIT |
| `sha2` | content/tree hashing | MIT/Apache-2.0 |
| `clap` | optional CLI (in `codeatlas-cli`, not core) | MIT/Apache-2.0 |

The **binding** crate adds only `pyo3` (MIT/Apache-2.0).

## Rationale
- Keeps the core small, auditable, and offline-buildable.
- Avoids heavy frameworks (tokio, reqwest, rusqlite, etc.) in v1 — the pipeline is synchronous,
  CPU-bound file analysis that does not need async or a database.

## Rules
1. Add a dependency only with a stated purpose + license compatibility; prefer permissive (MIT/
   Apache-2.0/BSD/ISC) licenses. No AGPL/GPL dependencies.
2. Any new transitive dependency must be license-compatible (audited via `cargo deny` before release).
3. tree-sitter grammars must be permissive-licensed (MIT/MIT/Apache-2.0); no GPL grammars.
4. The Rust core never depends on `pyo3`, network clients, or telemetry libraries.

## Consequences
- `cargo-deny` is added to CI before the first release (Phase 8).
- A minimal `codeatlas-cli` binary crate may be added for a standalone CLI, sharing the core, without
  pulling extra deps.