# Release Checklist

Gates that must pass before tagging the first public release. Status is refreshed
each time the project approaches a release; none of these steps publish anything
without explicit authorization.

## Build & tests
- [x] `cargo build --workspace` and `cargo test --workspace` pass on a clean checkout.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` is clean.
- [x] `cargo fmt --all --check` is clean.
- [x] `maturin build --release` produces a wheel; the wheel imports and `codeatlas
      full --local <fixture>` produces `analysis.json` end-to-end.
- [x] `pytest tests/` passes against the installed wheel.
- [ ] CI matrix (format, clippy, test, wheel build, pytest) verified green on PR.
- [x] `cargo deny check` (dependency licenses/advisories) passes.

## Feature & compatibility
- [ ] Implemented feature inventory matches reality (`docs/FEATURE_PARITY.md`).
- [ ] `analysis.json` output validated against an upstream sample where possible.
- [ ] CLI `--help`, README quick start, and code agree (no undocumented flags).

## Security & licensing
- [ ] No secrets/tokens in the repository or build artifacts.
- [ ] Dependency license audit complete (`cargo deny` + `docs/audit/DEPENDENCY_AND_LICENSE_AUDIT.md`).
- [ ] `LICENSE` and `NOTICE` present and accurate (attribution to CodeBoarding).

## Documentation & distribution
- [ ] README, CHANGELOG, CONTRIBUTING, and architecture docs up to date.
- [ ] Version in `Cargo.toml` + `pyproject.toml` bumped per SemVer.
- [ ] Wheel/package distribution strategy documented (manylinux + platform wheels).

## Remaining before a `0.2.0` tag
- [ ] Java/PHP/C# extractors (currently reported as `unsupported_language`).
- [x] Golden/snapshot test for `analysis.json` against a committed fixture
      (`crates/core/tests/golden.rs`).
- [ ] Golden/snapshot tests for `analysis.json` against a committed fixture.