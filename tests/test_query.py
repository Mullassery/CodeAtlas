"""Query API/CLI over the generated call-graph sidecar."""

from __future__ import annotations

import json

from codeatlas_rs.query import callers, callees, describe, load_call_graph, query_main

CG = {
    "nodes": [
        {
            "fully_qualified_name": "pkg.mod.helper",
            "file_path": "pkg/mod.py",
            "line_start": 1,
            "line_end": 2,
            "node_type": "FUNCTION",
        },
        {
            "fully_qualified_name": "pkg.mod.main",
            "file_path": "pkg/mod.py",
            "line_start": 4,
            "line_end": 7,
            "node_type": "FUNCTION",
        },
    ],
    "edges": [
        {
            "source": "pkg.mod.main",
            "destination": "pkg.mod.helper",
            "call_sites": [{"line": 5, "column": 9}, {"line": 6, "column": 9}],
        }
    ],
}


def test_callers_and_callees_by_simple_name():
    assert callers(CG, "helper")[0]["source"] == "pkg.mod.main"
    assert callees(CG, "main")[0]["destination"] == "pkg.mod.helper"


def test_describe_direction_filtering():
    both = describe(CG, "helper")
    assert both["symbols"][0]["fully_qualified_name"] == "pkg.mod.helper"
    assert "callers" in both and "callees" in both
    callers_only = describe(CG, "helper", direction="callers")
    assert "callees" not in callers_only


def test_load_from_repository_directory(tmp_path):
    out = tmp_path / ".codeatlas"
    out.mkdir()
    (out / "call_graph.json").write_text(json.dumps(CG), encoding="utf-8")
    assert load_call_graph(tmp_path)["nodes"][0]["fully_qualified_name"] == "pkg.mod.helper"


def test_query_main_json_output(tmp_path, capsys):
    path = tmp_path / "call_graph.json"
    path.write_text(json.dumps(CG), encoding="utf-8")
    query_main(path, "main", output_format="json")
    payload = json.loads(capsys.readouterr().out)
    assert payload["query"] == "main"
    assert payload["callees"][0]["destination"] == "pkg.mod.helper"
