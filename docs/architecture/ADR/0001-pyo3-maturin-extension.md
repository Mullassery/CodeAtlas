# ADR-0001: Ship the Rust core as a PyO3 + maturin native extension

- **Status:** Accepted
- **Date:** 2026-10-17

## Context
The project must be Rust-native but "wrapped in Python and saved to PyPI" (product constraint from
the owner). Two packaging strategies were considered for the Rust core:

1. **Standalone binary + subprocess** — Python shells out to a `codeatlas` binary over JSON/file
   contracts.
2. **PyO3 + maturin native extension** — Rust compiled into `codeatlas_rs._core`, imported in-process.

## Decision
Use **PyO3 + maturin** (option 2), confirmed by the owner.

## Rationale
- In-process calls avoid serialization and process-start overhead per analysis, and expose richer
  Python objects (not just JSON strings) for the wrapper's optional LLM/rendering orchestration.
- maturin provides reproducible wheel builds and a supported manylinux/CI story.
- The upstream tool is itself a Python package with a Python API (`run_full`, `build_generator`,
  `DiagramGenerator`); an extension keeps that API natural.

## Consequences
- The core library remains free of `pyo3`; a thin `codeatlas-py` binding crate isolates CPython ABI
  concerns (see ADR-0004).
- Build matrix now includes per-Python-version/OS wheels (maturin). Rust MSRV and PyO3's supported
  Python versions must be tracked together.
- Debugging spans the Python↔Rust boundary; the binding surface is kept minimal and explicit.