# Audit findings — 2026-10-19

Scope: the pending roadmap item **M9-4 "Java extractor"** in
`docs/PROJECT_BACKLOG.md` / `docs/IMPLEMENTATION_ROADMAP.md`. No other
roadmap items were started.

## What changed

- Added `crates/core/src/extract/java.rs` (tree-sitter Java):
  - classes, interfaces, enums, and records;
  - methods and constructors (`<init>`);
  - imports (including static imports);
  - `extends` / `implements` bases recorded for cross-file inheritance
    resolution;
  - `this`/unqualified method calls and `new KnownType()` constructor calls;
  - package declarations are used as the module prefix so `Foo.java` does not
    become `pkg.Foo.Foo`.
- Registered `Language::Java` in `extract/mod.rs`.
- Added `tree-sitter-java` to the workspace and core dependencies.
- Added unit tests and an end-to-end pipeline test for Java discovery,
  `methods_index`, and cross-file inheritance.
- Updated `README.md`, `docs/PROJECT_BACKLOG.md`, `docs/RELEASE_CHECKLIST.md`,
  `docs/IMPLEMENTATION_ROADMAP.md`, and `CHANGELOG.md`.

## Verification

```
cargo test -p codeatlas-core
# 37 lib tests + 1 golden test passed

cargo fmt --all --check
cargo clippy -p codeatlas-core --all-targets -- -D warnings
# clean
```

## Remaining work / follow-ups

- Java type resolution is still the same unique-simple-name heuristic as the
  other tree-sitter extractors; it is not LSP-level.
- Generic types, nested/anonymous classes, method references, and
  `super.method()` calls are not specially resolved.
- Kotlin, PHP, and C# remain unsupported.
- The `analysis.json` golden fixture does not yet include a Java file; the new
  pipeline test covers Java separately.
