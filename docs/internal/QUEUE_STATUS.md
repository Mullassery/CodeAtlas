# Queue status — 2026-10-19

This file records what was shipped, what is blocked/failed, and what is rejected for this repository. It complements `AUDIT_FINDINGS.md`.

## Shipped
- Java/PHP/C# extractors; architecture diff; PR action; CI import fix (0.7.0).
- Incremental whole-repository workflow cache (`codeatlas full --cache`).

## Blocked / failed
- Kotlin extractor (grammar incompatible with tree-sitter 0.25).

## Pending release verification
- The release workflow now builds, smoke-tests, and merges Linux/macOS/Windows
  wheels plus sdist, but PyPI 0.7.0 still only has a macOS arm64 wheel and the
  sdist. The remaining gap closes when a `v*` tag is released; do not claim the
  other wheels are published until then.

## Rejected / deferred
- Chat UI; LSP-level resolution now.

## Policy

Any future feature or upgrade must check `AUDIT_FINDINGS.md` and this file before implementation. A blocked item is retried only when its external dependency is available; a rejected item needs new user evidence or a design change to reopen.
