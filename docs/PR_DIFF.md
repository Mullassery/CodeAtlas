# PR architecture diff

`codeatlas diff` compares two `analysis.json` files. The composite action in
`.github/actions/architecture-diff` turns that into a PR check: it analyzes a
base checkout and a head checkout, writes a Markdown diff to the job summary,
and can fail the job on any change.

```yaml
name: architecture
on: pull_request
jobs:
  diff:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          path: head
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.base_ref }}
          path: base
      - uses: ./.github/actions/architecture-diff
        with:
          before-dir: base
          after-dir: head
          fail-on-change: "true"
```

Notes:

- The action installs `codeatlas-rs` from PyPI; it does not build the Rust
  extension from source.
- `fail-on-change: "true"` is strict: any component, relation, or method
  hash/location change fails the job. Start with `"false"` to collect signal.
- For a monorepo, pass the relevant subdirectory as both `before-dir` and
  `after-dir`.
