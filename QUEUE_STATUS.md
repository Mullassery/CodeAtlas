# Queue status — 2026-10-19

This file records what was shipped, what is blocked/failed, and what is rejected for this repository. It complements `AUDIT_FINDINGS.md`.

## Shipped
- Java/PHP/C# extractors; architecture diff; PR action; CI import fix (0.7.0).

## Blocked / failed
- Kotlin extractor (grammar incompatible with tree-sitter 0.25); Linux/Windows wheels.

## Rejected / deferred
- Chat UI; LSP-level resolution now.

## Policy

Any future feature or upgrade must check `AUDIT_FINDINGS.md` and this file before implementation. A blocked item is retried only when its external dependency is available; a rejected item needs new user evidence or a design change to reopen.
