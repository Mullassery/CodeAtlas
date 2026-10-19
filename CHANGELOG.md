# Changelog

All notable changes to CodeAtlas are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Release hardening: the release workflow now merges every wheel artifact
  into one `dist/` (`actions/download-artifact` with `merge-multiple`), fails
  if a build job produces no wheel (`if-no-files-found: error`), and refuses
  to publish unless the complete set is present (2 manylinux + 2 musllinux +
  macOS universal2 + Windows + one sdist).
- Wheel import smoke tests run before upload for every platform:
  `pip install dist/*.whl` then `python -c "import codeatlas_rs;
  print(codeatlas_rs.__version__)"`, including CPython 3.9 (the abi3 floor)
  and Alpine for the musllinux wheels. The sdist is installed and imported
  too.
- `.codeatlas/call_graph.json` sidecar: merged symbols plus call and reference
  edges, written by `codeatlas full` without changing the `analysis.json`
  schema.
- `codeatlas query <repo-or-json> <symbol>` with `--direction callers|callees`
  and `--format text|json` for local agents/editors.
- Python query helpers in `codeatlas_rs.query`.

### Fixed
- manylinux wheels imported with `undefined symbol: le16toh`. tree-sitter's C
  code needs glibc's `le16toh`/`be16toh` macros, which glibc 2.17 (the
  manylinux2014 toolchain) hides under `-std=c11` unless a feature-test macro
  is set; tree-sitter defines `_DEFAULT_SOURCE`, which glibc < 2.19 ignores.
  `.cargo/config.toml` now sets `CFLAGS=-D_GNU_SOURCE` for C dependencies, so
  manylinux and sdist builds on older glibc link cleanly.
- The `abi3-py39` wheel now imports on Python 3.9. `codeatlas_rs/__init__.py`
  used PEP 604 (`str | None`) annotations that were evaluated at import time;
  it now uses `from __future__ import annotations` like the other modules.

## [0.7.0] - 2026-10-19

### Added
- C# extraction via tree-sitter: namespaces, classes, interfaces, enums,
  records/structs, methods, constructors, `using` imports, base-list
  inheritance, `this`/unqualified calls, and `new` constructor edges. Unit
  tests cover symbols, calls, imports, and inheritance.

## [0.6.0] - 2026-10-19

### Added
- PHP extraction via tree-sitter: namespaces as module prefixes, classes,
  interfaces, traits, enums, methods, top-level functions, imports,
  `extends`/`implements` inheritance, `$this`/unqualified calls, and `new`
  constructor edges. Unit tests cover symbols, calls, imports, and inheritance.

## [0.5.1] - 2026-10-19

### Added
- `codeatlas diff BEFORE AFTER` compares two `analysis.json` files and reports
  added/removed components, component relations, added/removed methods, and
  changed method hashes/locations. Formats: text, Markdown, JSON.
- `--fail-on-change` exits `1` when anything changed, so a PR job can gate on
  architecture drift. Unit tests cover the diff and CLI exit behavior.

## [0.5.0] - 2026-10-19

### Added
- Java extraction via tree-sitter: classes, interfaces, enums and records;
  methods and constructors; imports; `extends`/`implements` inheritance edges
  (resolved across files via the existing class-base index); `this`/unqualified
  and known-type constructor call edges. Java package declarations are used as
  the module prefix, so `Foo.java` no longer produces `pkg.Foo.Foo`.
- Unit tests for the extractor plus an end-to-end pipeline test covering
  Java discovery, methods index entries, and cross-file class resolution.

## [0.3.0] - 2026-10-18

### Added
- Rust import extraction: `file_coverage.json`'s `imports` map is now populated
  for Rust sources (`use` declarations, including groups, aliases, globs and
  relative paths), bringing Rust to parity with Python/Go/TypeScript.

## [0.2.0] - 2026-10-18

### Added
- `file_coverage.json` now includes a per-file `imports` map (repo-relative
  path -> sorted dotted module names), enabling structural dependency and
  repository-boundary checks that previously required text scanning.
- Release workflow (`.github/workflows/release.yml`): manylinux/musllinux,
  macOS, and Windows wheels plus sdist, published via PyPI Trusted Publishing.

## [Unreleased]

## [0.4.0] - 2026-10-18

### Added
- Deterministic analysis core in Rust: repository discovery (gitignore +
  `.codeatlasignore`), content fingerprinting, and tree-sitter extraction
  for **Python**, **Rust**, **Go**, and **JavaScript/TypeScript** (symbols,
  qualified names, calls, imports, class/receiver hierarchies, cross-file
  unique-name call resolution). `.ts`/`.tsx` are auto-disambiguated by re-parsing
  with whichever grammar produces fewer parse errors.
- Deterministic directory-based component clustering with hierarchical component
  ids and per-scope (lowest-common-ancestor) relations.
- Schema-compatible `analysis.json` output plus `fingerprint.json` and
  `file_coverage.json` sidecars.
- Python package (`codeatlas-rs`, import `codeatlas_rs`) distributed via maturin/PyPI, with `codeatlas`
  and `codeatlas-render` CLIs.
- Renderers for Markdown + Mermaid, HTML (Cytoscape.js), MDX, and
  reStructuredText, with a render manifest that reconciles stale files and
  refuses to overwrite unowned files.
- Optional LLM component naming via OpenAI-compatible endpoints, with auth/quota
  exit codes (2/3) and deterministic-name fallback (ADR-0006).
- Security hardening: symlink and oversize-file skipping; hostile-content tests.
- `codeatlas-bundle` (new `packages/codeatlas-bundle`): a pure-Python package that
  runs the analysis, renders docs, and commits them into a repository, with
  `init`/`build`/`check` commands and a `codeatlas.json` opt-in file.
- `analyze()` accepts `ignore_paths` so generated output directories can be
  excluded from discovery.

### Changed
- Relations are scoped to their owning (lowest-common-ancestor) component rather
  than flattened to the root scope.

### Known limitations
- Tree-sitter resolution is intentionally simpler than an LSP pipeline;
  see `docs/audit/KNOWN_GAPS.md`.
- Clustering is a simplified deterministic directory partition (full
  affinity/kinship + TreeSpec replay deferred to incremental support).
- Java, PHP, C#, and Kotlin parsers not yet implemented (recognized in
  discovery, recorded as `unsupported_language` in coverage).