"""Content-addressed analysis cache.

The Rust pipeline is deterministic. This cache keys an entire analysis by a
fingerprint of the repository contents (git HEAD/status when available, plus
file path/size/mtime metadata) and returns the previous analysis when nothing
changed. It is a workflow cache, not a partial per-file re-analysis engine.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path

SOURCE_SUFFIXES = {
    ".py", ".rs", ".go", ".js", ".jsx", ".ts", ".tsx", ".java", ".php", ".cs",
}


def _git_metadata(root: Path) -> str:
    try:
        head = subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            stderr=subprocess.DEVNULL,
            text=True,
        ).strip()
        status = subprocess.check_output(
            ["git", "-C", str(root), "status", "--porcelain"],
            stderr=subprocess.DEVNULL,
            text=True,
        )
        return head + "\n" + status
    except (OSError, subprocess.CalledProcessError):
        return ""


def repo_fingerprint(root: str | Path, ignore_paths: list[str] | None = None) -> str:
    root = Path(root).resolve()
    ignored = {Path(p).resolve() for p in (ignore_paths or [])}
    hasher = hashlib.sha256()
    hasher.update(_git_metadata(root).encode("utf-8", "replace"))
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix.lower() not in SOURCE_SUFFIXES:
            continue
        if any(ignored_path in path.parents for ignored_path in ignored):
            continue
        if any(part in {".git", "target", "node_modules", ".venv", "__pycache__"} for part in path.parts):
            continue
        stat = path.stat()
        hasher.update(str(path.relative_to(root)).encode())
        hasher.update(str(stat.st_size).encode())
        hasher.update(str(int(stat.st_mtime_ns)).encode())
    return hasher.hexdigest()


@dataclass
class CacheEntry:
    key: str
    analysis_json: str
    fingerprint_json: str
    file_coverage_json: str


class AnalysisCache:
    def __init__(self, cache_dir: str | Path):
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)

    def _path(self, key: str) -> Path:
        return self.cache_dir / f"{key}.json"

    def load(self, key: str) -> CacheEntry | None:
        path = self._path(key)
        if not path.is_file():
            return None
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return None
        required = ("analysis_json", "fingerprint_json", "file_coverage_json")
        if not all(isinstance(data.get(field), str) for field in required):
            return None
        return CacheEntry(key=key, **{field: data[field] for field in required})

    def store(self, entry: CacheEntry) -> None:
        self._path(entry.key).write_text(
            json.dumps(
                {
                    "analysis_json": entry.analysis_json,
                    "fingerprint_json": entry.fingerprint_json,
                    "file_coverage_json": entry.file_coverage_json,
                }
            ),
            encoding="utf-8",
        )


__all__ = ["AnalysisCache", "CacheEntry", "repo_fingerprint"]
