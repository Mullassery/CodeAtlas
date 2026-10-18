"""Tests for codeatlas-bundle: generation, staleness checks, and commits."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import pytest

from codeatlas_bundle import bundle, check, detect_repo_root, init_config, load_config


def _make_repo(tmp_path: Path) -> Path:
    (tmp_path / "app").mkdir()
    (tmp_path / "core").mkdir()
    (tmp_path / "app" / "main.py").write_text("def run():\n    return helper()\n")
    (tmp_path / "app" / "helper.py").write_text("def helper():\n    return 1\n")
    (tmp_path / "core" / "engine.py").write_text("def start():\n    return 1\n")
    return tmp_path


def _git(repo: Path, *args: str) -> None:
    subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)


def test_init_writes_config(tmp_path):
    repo = _make_repo(tmp_path)
    path = init_config(repo)
    assert path == repo / "codeatlas.json"
    data = json.loads(path.read_text())
    assert data["output_dir"] == "docs/codeatlas"
    assert data["formats"] == ["markdown", "html"]

    with pytest.raises(FileExistsError):
        init_config(repo)
    assert init_config(repo, force=True).exists()


def test_bundle_generates_docs_and_is_current(tmp_path):
    repo = _make_repo(tmp_path)
    init_config(repo)

    result = bundle(repo)
    rels = {p.relative_to(repo).as_posix() for p in result.files}
    assert "docs/codeatlas/overview.md" in rels
    assert "docs/codeatlas/overview.html" in rels

    current, changed = check(repo)
    assert current, changed
    assert changed == []


def test_check_detects_source_change(tmp_path):
    repo = _make_repo(tmp_path)
    init_config(repo)
    bundle(repo)

    # Change the code without regenerating the bundle.
    (repo / "app" / "helper.py").write_text("def helper():\n    return 2\n")
    (repo / "app" / "new.py").write_text("def added():\n    return 3\n")

    current, changed = check(repo)
    assert not current
    assert changed


def test_bundle_commits_into_repo(tmp_path):
    repo = _make_repo(tmp_path)
    _git(repo, "init")
    _git(repo, "config", "user.email", "test@example.com")
    _git(repo, "config", "user.name", "Test")
    _git(repo, "add", "-A")
    _git(repo, "commit", "-m", "initial")

    init_config(repo)
    result = bundle(repo, commit=True)
    assert result.committed is True

    log = subprocess.run(
        ["git", "-C", str(repo), "log", "--oneline"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    assert "codeatlas" in log

    # No changes on a second run -> nothing to commit.
    result2 = bundle(repo, commit=True)
    assert result2.committed is False


def test_detect_repo_root_finds_config(tmp_path):
    repo = _make_repo(tmp_path)
    init_config(repo)
    nested = repo / "app"
    assert detect_repo_root(nested) == repo


def test_load_config_defaults_without_file(tmp_path):
    repo = _make_repo(tmp_path)
    config = load_config(repo)
    assert config.project_name == repo.name
    assert config.output_dir == "docs/codeatlas"
