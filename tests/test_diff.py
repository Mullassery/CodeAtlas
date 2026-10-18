"""Tests for the analysis.json diff command."""

import importlib.util
import json
import sys
from pathlib import Path

import pytest

# Load diff.py directly: the package __init__ imports the compiled Rust
# extension, which is absent in a source-only test environment.
_DIFF_PATH = Path(__file__).resolve().parents[1] / "python" / "codeatlas_rs" / "diff.py"
_spec = importlib.util.spec_from_file_location("codeatlas_diff_under_test", _DIFF_PATH)
assert _spec and _spec.loader
_diff = importlib.util.module_from_spec(_spec)
sys.modules[_spec.name] = _diff
_spec.loader.exec_module(_diff)
diff_analyses = _diff.diff_analyses
render_markdown = _diff.render_markdown
render_text = _diff.render_text


def _analysis(components, relations, methods):
    return {
        "components": [{"component_id": cid, "name": name} for cid, name in components],
        "components_relations": [
            {"src_name": src, "dst_name": dst, "relation": "imports"}
            for src, dst in relations
        ],
        "methods_index": {
            key: {"content_hash": value, "start_line": 1, "end_line": 2, "type": "FUNCTION"}
            for key, value in methods.items()
        },
    }


def test_detects_component_relation_and_method_changes():
    before = _analysis(
        [("1", "app"), ("2", "core")],
        [("app", "core")],
        {"app/main.py|app.run": "hash-a"},
    )
    after = _analysis(
        [("1", "app"), ("3", "web")],
        [("web", "app")],
        {"app/main.py|app.run": "hash-b", "web/api.py|web.serve": "hash-c"},
    )

    diff = diff_analyses(before, after)
    assert diff.has_changes
    assert diff.added_components == ("web",)
    assert diff.removed_components == ("core",)
    assert diff.removed_relations == ("app -> core (imports)",)
    assert diff.added_relations == ("web -> app (imports)",)
    assert diff.added_methods == ("web/api.py|web.serve",)
    assert diff.changed_methods == ("app/main.py|app.run",)


def test_identical_analyses_have_no_changes():
    analysis = _analysis([("1", "app")], [], {"app/main.py|app.run": "h"})
    diff = diff_analyses(analysis, analysis)
    assert not diff.has_changes
    assert render_text(diff) == "No architecture changes."
    assert render_markdown(diff) == "No architecture changes."


def test_renders_markdown_and_json_payload():
    before = _analysis([("1", "app")], [], {})
    after = _analysis([("1", "app"), ("2", "web")], [], {})
    diff = diff_analyses(before, after)

    assert "web" in render_text(diff)
    assert "Added components" in render_markdown(diff)
    assert diff.to_dict()["added_components"] == ["web"]


def test_cli_fail_on_change_exits_one(tmp_path):
    try:
        from codeatlas_rs.cli import main
    except ImportError:
        pytest.skip("compiled codeatlas_rs._core extension is not built")

    before = tmp_path / "before.json"
    after = tmp_path / "after.json"
    before.write_text(json.dumps(_analysis([("1", "app")], [], {})), encoding="utf-8")
    after.write_text(
        json.dumps(_analysis([("1", "app"), ("2", "web")], [], {})), encoding="utf-8"
    )

    with pytest.raises(SystemExit) as exc:
        main(["diff", str(before), str(after), "--format", "json", "--fail-on-change"])
    assert exc.value.code == 1
