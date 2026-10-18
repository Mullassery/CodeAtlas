# Project Backlog

> 2026-10-19 implementation findings: see [`../AUDIT_FINDINGS.md`](../AUDIT_FINDINGS.md).

Lightweight task tracking. IDs are stable; status ∈ `todo` | `doing` | `done` | `blocked`.
Acceptance criteria (AC) and test evidence are recorded as they complete.

> Decisions are tracked separately in `docs/architecture/ADR/`. See `docs/IMPLEMENTATION_ROADMAP.md`
> for milestone grouping.

## Milestone M0 — Project skeleton
| ID | Task | Status | AC / Evidence |
|---|---|---|---|
| M0-1 | Cargo workspace (core + py), maturin mixed layout, `.gitignore`, LICENSE/NOTICE | done | `cargo build --workspace` OK (with PYO3_PYTHON) |
| M0-2 | PyO3 binding `codeatlas_rs._core.version()` | done | `maturin build` → wheel imports; `codeatlas --version` prints `0.1.0` |
| M0-3 | Rust unit tests run | done | `cargo test -p codeatlas-core` → 2 passed |
| M0-4 | CI (fmt, clippy, test, build) | done | `.github/workflows/ci.yml` |

## Milestone M1 — Domain models & serialization
| ID | Task | Status | AC |
|---|---|---|---|
| M1-1 | Full `analysis.json` serde model (metadata, files, methods_index, components, relations) | done | golden serialization matches upstream schema |
| M1-2 | Component-id assignment (`1`, `1.2`, …) + method-key convention | done | unit tests |
| M1-3 | `fingerprint.json` + tree hash | done | deterministic hash tests |

## Milestone M2 — Discovery & ignore
| ID | Task | Status | AC |
|---|---|---|---|
| M2-1 | Suffix→language map + file walker | done | fixture discovery tests |
| M2-2 | gitwildmatch ignore (`.gitignore` + `.codeatlasignore`) | done | ignore fixtures |

## Milestone M3 — Tree-sitter analysis (Python first)
| ID | Task | Status | AC |
|---|---|---|---|
| M3-1 | Python extractor: symbols, qualified names, spans | done | golden graph |
| M3-2 | Python calls + imports + `class X(Base)` | done | unit tests |

## Milestone M4 — Clustering
| ID | Task | Status | AC |
|---|---|---|---|
| M4-1 | Deterministic hierarchical partition (directory-based; simplified vs upstream affinity/kinship) | done | fixture partition tests |
| M4-2 | TreeSpec serialization + replay | todo | deferred to incremental (P2) |

## Milestone M5 — End-to-end vertical slice
| ID | Task | Status | AC |
|---|---|---|---|
| M5-1 | Python wrapper `codeatlas full --local` → analysis.json + fingerprint.json | done | fixture run asserts output |
| M5-2 | Deterministic names without LLM | done | no-provider run succeeds |
| M5-3 | Cross-file call resolution (unique-name heuristic) + component relations | done | pipeline test |
| M5-4 | `file_coverage.json` artifact | done | coverage sidecar + tests |

## Milestone M6 — Rendering
| ID | Task | Status | AC |
|---|---|---|---|
| M6-1 | md/mdx/html/rst generators + mermaid/cytoscape | done | pytest golden checks |
| M6-2 | Render manifest + overwrite protection | done | pytest tests |

## Milestone M7 — Optional LLM enrichment
| ID | Task | Status | AC |
|---|---|---|---|
| M7-1 | Provider facade + naming/description (OpenAI-compatible) | done | mock-provider tests |
| M7-2 | Auth/quota exit codes 2/3 | done | error-mapping tests |

## Milestone M8 — Hardening
| ID | Task | Status | AC |
|---|---|---|---|
| M8-1 | Security: symlink/path traversal, size limits, no exec | done | symlink/size/hostile-content tests |
| M8-2 | Error-path tests, benchmarks | doing | green suite, clippy clean |

## Milestone M9 — Additional languages
| ID | Task | Status | AC |
|---|---|---|---|
| M9-1 | Rust tree-sitter extractor | done | unit tests + self-dogfood |
| M9-2 | Go extractor | done | unit tests |
| M9-3 | JavaScript/TypeScript extractor | done | TS/TSX auto-fallback, unit tests |
| M9-4 | Java extractor | done | classes/interfaces/enums/records, methods/constructors, imports, inheritance; unit + pipeline tests |
| M9-5 | PHP extractor | done | namespaces, classes/interfaces/traits/enums, functions/methods, imports, inheritance, calls |

## Milestone M10 — Regression safety
| ID | Task | Status | AC |
|---|---|---|---|
| M10-1 | Golden `analysis.json` snapshot on a committed multi-language fixture | done | `crates/core/tests/golden.rs` |

## Milestone M12 — Architecture diff
| ID | Task | Status | AC |
|---|---|---|---|
| M12-1 | `codeatlas diff` for two `analysis.json` files (components, relations, methods) | done | unit tests + text/Markdown/JSON |
| M12-2 | `--fail-on-change` CI exit code | done | CLI test |

## Milestone M11 — Cross-repo bundling
| ID | Task | Status | AC |
|---|---|---|---|
| M11-1 | `codeatlas-bundle` package (`init`/`build`/`check`, commit) | done | pytest (6 tests) |
| M11-2 | Core `ignore_paths` so the bundle dir is excluded from discovery | done | pipeline tests |
| M11-3 | Pilot onboarding across Mullassery repos | done | 5 PRs (`docs/CROSS_REPO_ONBOARDING.md`) |

## Risks
- LSP-level resolution fidelity (G1) — tracked in `docs/audit/KNOWN_GAPS.md`.
- No private upstream integration fixtures (G10) — build own fixtures.