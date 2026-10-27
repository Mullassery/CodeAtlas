# Cross-repo onboarding

`codeatlas-bundle` lets any repository version a CodeAtlas architecture map
alongside its code. This page tracks the pilot rollout across the
`Mullassery` GitHub organization.

## How a repo connects

1. `pip install codeatlas-rs codeatlas-bundle`
2. `codeatlas-bundle init` — writes `codeatlas.json`.
3. `codeatlas-bundle build --commit` — generates and commits `docs/codeatlas/`.
4. Add the `.github/workflows/codeatlas.yml` workflow so the map refreshes on
   each push to `main` (or use `codeatlas-bundle check` in CI to gate staleness).

The generated docs live under `docs/codeatlas/` and are excluded from the
analysis (`ignore_paths`), so repeated runs are idempotent.

## Pilot set

Language reflects the repository's primary language; all were generated with the
current `codeatlas-bundle` and pass `codeatlas-bundle check`.

| Repository | Language | PR |
|---|---|---|
| [PyPerfGuard](../PyPerfGuard) | Rust | [#1](../PyPerfGuard/pull/1) |
| [PyContextFold](../PyContextFold) | Python/Rust | [#1](../PyContextFold/pull/1) |
| [PyTaskWireCheck](../PyTaskWireCheck) | Python | [#5](../PyTaskWireCheck/pull/5) |
| [PyDeepMesh](../PyDeepMesh) | Python | [#1](../PyDeepMesh/pull/1) |
| [PyAPICheck](../PyAPICheck) | Rust | [#7](../PyAPICheck/pull/7) |

## Follow-ups

- Publish `codeatlas` and `codeatlas-bundle` to PyPI so the CI workflow can
  `pip install` instead of installing from git + building the Rust extension.
- Offer a reusable GitHub Actions workflow so callers add a ~6-line caller
  instead of a full job.
- Roll out to the remaining non-archived repos once the pilot is merged.
