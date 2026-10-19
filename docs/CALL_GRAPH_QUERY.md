# Call graph sidecar and querying

`codeatlas full` writes the merged symbol/call graph to:

```text
<repo>/.codeatlas/call_graph.json
```

`analysis.json` keeps its established schema; the graph is a sidecar so local
agents and editors can query precise symbol context without re-running
analysis.

## Schema

```jsonc
{
  "nodes": [
    {
      "fully_qualified_name": "pkg.mod.helper",
      "file_path": "pkg/mod.py",
      "line_start": 1,
      "line_end": 2,
      "col_start": 1,
      "node_type": "Function"
    }
  ],
  "edges": [
    {
      "source": "pkg.mod.main",
      "destination": "pkg.mod.helper",
      "call_sites": [{ "line": 5, "column": 9 }]
    }
  ],
  "reference_edges": [
    { "src": "...", "dst": "...", "kind": "Import" }
  ]
}
```

- `nodes` are symbols with source locations.
- `edges` are **call** edges; each call site produces an edge (or a call-site
  entry on an edge).
- `reference_edges` carry non-call relations (`Contains`, `Inherits`,
  `Typeref`, `Import`).

## CLI

```bash
# Simple name or fully qualified name
codeatlas query /path/to/repo helper
codeatlas query /path/to/repo helper --direction callers
codeatlas query /path/to/repo helper --direction callees --format json

# Point at the sidecar directly
codeatlas query /path/to/repo/.codeatlas/call_graph.json helper
```

Text output prints the symbol location, then callers/callees with call-site
line numbers. JSON output returns the full symbols/edges structure.

## Python API

```python
from codeatlas_rs.query import load_call_graph, callers, callees, describe

cg = load_call_graph("/path/to/repo")        # or a call_graph.json path
describe(cg, "helper", direction="both")     # {"symbols": [...], "callers": [...], "callees": [...]}
```

Resolution order: exact `fully_qualified_name`, then simple-name suffix match
(e.g. `helper` matches `pkg.mod.helper`).

## Limits

- Static analysis only: dynamic dispatch, decorators, and reflection are not
  resolved.
- `callers`/`callees` use **call edges** only; use `reference_edges` for
  imports/inheritance/type references.
- Cross-language resolution follows CodeAtlas's normal extractor limits (see
  `docs/FEATURE_PARITY.md`).
- The sidecar is produced by `full`; `--cache` reuses the existing sidecar when
  analysis is unchanged.
