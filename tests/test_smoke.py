import json

import codeatlas_rs


def _fixture(tmp_path):
    (tmp_path / "app").mkdir()
    (tmp_path / "core").mkdir()
    (tmp_path / "app" / "main.py").write_text("def run():\n    return helper()\n")
    (tmp_path / "app" / "util.py").write_text("def helper():\n    return 42\n")
    (tmp_path / "core" / "engine.py").write_text("def start():\n    return run()\n")


def test_analyze_smoke(tmp_path):
    _fixture(tmp_path)
    analysis_json, fingerprint_json, file_coverage_json = codeatlas_rs.analyze(
        str(tmp_path), project_name="demo"
    )

    analysis = json.loads(analysis_json)
    assert analysis["metadata"]["repo_name"] == "demo"
    assert analysis["metadata"]["depth_cap"] == 3

    names = [c["name"] for c in analysis["components"]]
    assert names == ["app", "core"]

    relation = analysis["components_relations"][0]
    assert relation["src_name"] == "core"
    assert relation["dst_name"] == "app"
    assert relation["is_static"] is True

    methods = list(analysis["methods_index"].keys())
    assert "app/main.py|app.main.run" in methods
    assert "core/engine.py|core.engine.start" in methods

    fingerprint = json.loads(fingerprint_json)
    assert "app/main.py" in fingerprint["files"]
    assert "core/engine.py" in fingerprint["files"]

    coverage = json.loads(file_coverage_json)
    assert coverage["version"] == 1
    assert coverage["summary"]["analyzed"] >= 3


def test_version():
    assert isinstance(codeatlas_rs.__version__, str)
    assert codeatlas_rs.__version__


def test_analyze_javascript_typescript(tmp_path):
    (tmp_path / "web").mkdir()
    (tmp_path / "web" / "service.ts").write_text(
        "export class Service {\n"
        "  run(): number {\n"
        "    return this.help();\n"
        "  }\n"
        "  help(): number {\n"
        "    return 42;\n"
        "  }\n"
        "}\n"
    )
    (tmp_path / "web" / "main.js").write_text(
        "function boot() {\n  const s = new Service();\n  return s.run();\n}\n"
    )

    analysis_json, _, file_coverage_json = codeatlas_rs.analyze(
        str(tmp_path), project_name="web"
    )
    analysis = json.loads(analysis_json)
    methods = list(analysis["methods_index"].keys())
    assert "web/service.ts|web.service.Service" in methods
    assert "web/service.ts|web.service.Service.run" in methods
    assert "web/main.js|web.main.boot" in methods

    coverage = json.loads(file_coverage_json)
    assert coverage["summary"]["analyzed"] >= 2
    reasons = [
        f["reason"] for f in coverage["not_analyzed_files"]
    ]
    assert "unsupported_language" not in reasons

