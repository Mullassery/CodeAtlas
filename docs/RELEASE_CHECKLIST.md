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
- [x] CI matrix (format, clippy, test, wheel build, pytest) verified green (runs on `main`).
- [x] `cargo deny check` (dependency licenses/advisories) passes.

## Feature & compatibility
- [ ] Implemented feature inventory matches reality (`docs/FEATURE_PARITY.md`).
- [ ] `analysis.json` output validated against an upstream sample where possible.
- [ ] CLI `--help`, README quick start, and code agree (no undocumented flags).

## Security & licensing
- [ ] No secrets/tokens in the repository or build artifacts.
- [ ] Dependency license audit complete (`cargo deny`).
- [ ] `LICENSE` and `NOTICE` present and accurate.

## Documentation & distribution
- [ ] README, CHANGELOG, CONTRIBUTING, and architecture docs up to date.
- [ ] Version in `Cargo.toml` + `pyproject.toml` bumped per SemVer.
- [x] Wheel/package distribution strategy documented (manylinux + platform wheels).

## Release pipeline (`.github/workflows/release.yml`)
- [x] Matrix builds manylinux x86_64/aarch64, musllinux x86_64/aarch64, macOS
      universal2, Windows x86_64, and an sdist.
- [x] Each build job fails if it produces no wheel/sdist
      (`if-no-files-found: error`).
- [x] Every wheel is installed from `dist/*.whl` and imported
      (`import codeatlas_rs; print(codeatlas_rs.__version__)`) before upload;
      musllinux wheels are imported in an Alpine container and the abi3 wheel
      is checked on CPython 3.9. The sdist is installed and imported too.
- [x] Publish job downloads all artifacts with `merge-multiple: true`, asserts
      the complete set (6 wheels + 1 sdist), and checks the tag matches
      `pyproject.toml` before invoking PyPI Trusted Publishing.
- [ ] First release after the pipeline fix verified end-to-end on PyPI. 0.7.0
      predates the fix and only has a macOS arm64 wheel + sdist; the other
      wheels are not published until a `v*` tag runs this workflow.

## Remaining before a `0.2.0` tag
- [x] Java, PHP, and C# extractors.
- [x] Golden/snapshot test for `analysis.json` against a committed fixture
      (`crates/core/tests/golden.rs`).
- [ ] Golden/snapshot tests for `analysis.json` against a committed fixture.