import json

from codeatlas_rs.render import render


def _analysis(tmp_path):
    (tmp_path / "app").mkdir()
    (tmp_path / "core").mkdir()
    (tmp_path / "app" / "main.py").write_text("def run():\n    return helper()\n")
    (tmp_path / "app" / "util.py").write_text("def helper():\n    return 42\n")
    (tmp_path / "core" / "engine.py").write_text("def start():\n    return run()\n")

    import codeatlas_rs

    analysis_json, _, _ = codeatlas_rs.analyze(str(tmp_path), project_name="demo")
    analysis_path = tmp_path / "analysis.json"
    analysis_path.write_text(analysis_json)
    return analysis_path, json.loads(analysis_json)


def test_render_markdown(tmp_path):
    analysis_path, _ = _analysis(tmp_path)
    out = tmp_path / "out"
    written = render("md", analysis_path, out)

    overview = out / "overview.md"
    assert overview in written
    content = overview.read_text()
    assert "```mermaid" in content
    assert 'app["app"]' in content
    assert "calls" in content

    manifest = json.loads((out / ".codeatlas-render.json").read_text())
    assert manifest["version"] == 1
    assert "overview.md" in manifest["outputs"]["md"]


def test_render_does_not_overwrite_unowned(tmp_path):
    analysis_path, _ = _analysis(tmp_path)
    out = tmp_path / "out"
    out.mkdir()
    (out / "overview.md").write_text("user content")

    try:
        render("md", analysis_path, out)
    except FileExistsError:
        return
    raise AssertionError("expected FileExistsError for unowned file")
