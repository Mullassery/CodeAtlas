# ADR-0006: Deterministic naming fallback when no LLM is configured

- **Status:** Accepted
- **Date:** 2026-10-17

## Context
Upstream CodeBoarding aborts a full run when no LLM provider is configured — it refuses to ship
folder-named components because they "look like a finished diagram". The v1 target is the
**deterministic analysis core first** (owner decision), which must be usable without network/keys.

## Decision
The Rust core always produces a complete, deterministic analysis. Component naming follows this
order:

1. When a provider is configured, the Python wrapper asks it for `name`, `description`,
   `key_entities`, and relation wording.
2. When no provider is configured (or the provider fails in a recoverable way), the wrapper uses
   deterministic, folder/name-derived component names and a generated description (mirroring
   upstream's `_fallback_description`), and marks the run as "deterministic/non-semantic" in metadata.

## Rationale
- The core value (structure, components, dependencies, diagrams) works offline and is fully testable.
- Exposes a wider, verifiable surface than upstream's hard-fail path, while still labeling the
  result as not-semantically-enriched so consumers can tell the difference.

## Consequences
- This is a **deliberate divergence** from upstream (documented in G2). Upstream's "fail rather than
  mislabel" policy is respected *by labeling* rather than by aborting.
- `metadata` gains a `naming_mode` field (`"semantic"` | `"deterministic"`); this is additive and does
  not break schema readers (upstream ignores unknown fields).
- Auth/quota errors (upstream exit 2/3) remain terminal in the wrapper when a provider was explicitly
  requested, so half-configured runs don't silently degrade.