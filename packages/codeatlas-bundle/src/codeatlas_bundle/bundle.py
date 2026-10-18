"""Bundle CodeAtlas architecture docs into a target repository.

`codeatlas-bundle` runs the deterministic analysis, renders the docs, writes
them under the configured output directory, and (optionally) commits the result
so the architecture map is versioned alongside the code.
"""

from __future__ import annotations

import json
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

import codeatlas
from codeatlas.render import render

from codeatlas_bundle.config import CONFIG_FILENAME, Config, load_config

ANALYSIS_FILENAME = "analysis.json"
FINGERPRINT_FILENAME = "fingerprint.json"
FILE_COVERAGE_FILENAME = "file_coverage.json"
DEFAULT_COMMIT_MESSAGE = "docs(codeatlas): refresh architecture map"


@dataclass
class BundleResult:
    repo_root: Path
    output_dir: Path
    files: list[Path] = field(default_factory=list)
    committed: bool = False


def detect_repo_root(start: Path | str | None = None) -> Path:
    """Walk up from `start` to the nearest directory containing `.git` or
    `codeatlas.json`; fall back to `start` itself."""
    current = Path(start or Path.cwd()).resolve()
    for candidate in [current, *current.parents]:
        if (candidate / ".git").exists() or (candidate / CONFIG_FILENAME).exists():
            return candidate
    return current


def bundle(
    repo_root: Path | str,
    config: Config | None = None,
    *,
    commit: bool = False,
    message: str | None = None,
) -> BundleResult:
    """Generate the architecture bundle for `repo_root` into its output dir."""
    root = Path(repo_root).resolve()
    cfg = config or load_config(root)
    out_dir = root / cfg.output_dir

    files = _generate(root, cfg, out_dir)
    result = BundleResult(repo_root=root, output_dir=out_dir, files=files)

    if commit:
        result.committed = _commit(root, out_dir, message or DEFAULT_COMMIT_MESSAGE)
    return result


def check(repo_root: Path | str, config: Config | None = None) -> tuple[bool, list[str]]:
    """Return `(is_current, changed_files)` comparing the committed bundle with
    a freshly generated one."""
    root = Path(repo_root).resolve()
    cfg = config or load_config(root)
    committed_dir = root / cfg.output_dir

    changed: list[str] = []
    with tempfile.TemporaryDirectory(prefix="codeatlas-bundle-") as tmp:
        generated_dir = Path(tmp) / "out"
        _generate(root, cfg, generated_dir)

        generated = _relative_files(generated_dir)
        committed = _relative_files(committed_dir) if committed_dir.is_dir() else {}

        for rel, path in generated.items():
            committed_path = committed_dir / rel
            if rel not in committed or _content(path) != _content(committed_path):
                changed.append(rel)
        for rel in committed:
            if rel not in generated:
                changed.append(rel)

    return (not changed, sorted(changed))


def _generate(root: Path, cfg: Config, out_dir: Path) -> list[Path]:
    out_dir.mkdir(parents=True, exist_ok=True)
    # Always exclude the repository's configured output dir from analysis, even
    # when writing to a temporary directory (as `check` does).
    repo_output = root / cfg.output_dir
    analysis_json, fingerprint_json, coverage_json = codeatlas.analyze(
        str(root),
        project_name=cfg.project_name,
        depth_cap=cfg.depth_cap,
        ignore_paths=[str(repo_output)],
    )

    written: list[Path] = []
    analysis_path = out_dir / ANALYSIS_FILENAME
    analysis_path.write_text(_pretty(analysis_json), encoding="utf-8")

    if cfg.include_analysis:
        (out_dir / FINGERPRINT_FILENAME).write_text(fingerprint_json, encoding="utf-8")
        (out_dir / FILE_COVERAGE_FILENAME).write_text(coverage_json, encoding="utf-8")
        written += [
            analysis_path,
            out_dir / FINGERPRINT_FILENAME,
            out_dir / FILE_COVERAGE_FILENAME,
        ]

    for code in cfg.format_codes:
        written.extend(render(code, analysis_path, out_dir))

    if not cfg.include_analysis:
        analysis_path.unlink(missing_ok=True)

    return sorted({p for p in written})


def _pretty(raw_json: str) -> str:
    return json.dumps(json.loads(raw_json), indent=2) + "\n"


def _relative_files(directory: Path) -> dict[str, Path]:
    files: dict[str, Path] = {}
    for path in sorted(directory.rglob("*")):
        if path.is_file():
            files[str(path.relative_to(directory))] = path
    return files


def _content(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    if path.name == ANALYSIS_FILENAME:
        data = json.loads(text)
        data.get("metadata", {}).pop("generated_at", None)
        return json.dumps(data, sort_keys=True)
    return text


def _commit(root: Path, out_dir: Path, message: str) -> bool:
    config_path = root / CONFIG_FILENAME
    targets = [str(out_dir)]
    if config_path.exists():
        targets.append(str(config_path))

    _git(root, "add", "--", *targets)
    if _git(root, "diff", "--cached", "--quiet").returncode == 0:
        return False

    identity = _git(root, "config", "user.email").stdout.strip()
    prefix: list[str] = []
    if not identity:
        prefix = [
            "-c",
            "user.name=codeatlas-bundle",
            "-c",
            "user.email=codeatlas-bundle@users.noreply.github.com",
        ]
    _git(root, *prefix, "commit", "-m", message, check=True)
    return True


def _git(root: Path, *args: str, check: bool = False) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["git", "-C", str(root), *args],
        capture_output=True,
        text=True,
    )
    if check and result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(args)} failed: {result.stderr.strip() or result.stdout.strip()}"
        )
    return result
