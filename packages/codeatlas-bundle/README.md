# codeatlas-bundle

Bundle [CodeAtlas](../CodeAtlas) architecture maps
into any repository so the map is versioned alongside the code.

`codeatlas-bundle` runs the deterministic CodeAtlas analysis, renders the docs
(Markdown + HTML by default), writes them under a configured directory, and can
commit the result. It is pure Python and works for repositories in any language
CodeAtlas supports (Python, Rust, Go, JavaScript/TypeScript).

## Install

```bash
pip install codeatlas-rs codeatlas-bundle
```

## Connect a repository

```bash
cd my-repo
codeatlas-bundle init          # writes codeatlas.json
codeatlas-bundle build         # generates docs/codeatlas/
codeatlas-bundle build --commit  # generate and commit
```

`codeatlas.json`:

```json
{
  "project_name": "my-repo",
  "output_dir": "docs/codeatlas",
  "formats": ["markdown", "html"],
  "depth_cap": 3,
  "include_analysis": false
}
```

- `formats`: any of `markdown`, `html`, `mdx`, `rst`.
- `include_analysis`: when `true`, also commit the raw `analysis.json`,
  `fingerprint.json`, and `file_coverage.json` sidecars (the raw analysis
  contains a generation timestamp, so this produces a larger diff each run).

## CI: keep the map fresh

```yaml
# .github/workflows/codeatlas.yml
name: CodeAtlas
on:
  push:
    branches: [main]
permissions:
  contents: write
jobs:
  bundle:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - run: pip install codeatlas-rs codeatlas-bundle
      - run: git config user.name "codeatlas" && git config user.email "codeatlas@users.noreply.github.com"
      - run: codeatlas-bundle build --commit
      - run: git push
```

To gate a pull request instead of committing, use `codeatlas-bundle check`,
which exits non-zero when the committed bundle is stale.

## Python API

```python
from codeatlas_bundle import bundle, check

result = bundle("path/to/repo", commit=True)
print([str(p) for p in result.files])

is_current, changed = check("path/to/repo")
```
