"""Configuration for `codeatlas-bundle`.

A target repository opts in by committing a `codeatlas.json` file at its root.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

CONFIG_FILENAME = "codeatlas.json"
DEFAULT_OUTPUT_DIR = "docs/codeatlas"
DEFAULT_FORMATS = ("markdown", "html")

# Friendly format names -> `codeatlas.render` format codes.
FORMAT_CODES = {
    "markdown": "md",
    "md": "md",
    "html": "html",
    "mdx": "mdx",
    "rst": "rst",
}


@dataclass
class Config:
    """Resolved bundle configuration."""

    project_name: str
    output_dir: str = DEFAULT_OUTPUT_DIR
    formats: list[str] = field(default_factory=lambda: list(DEFAULT_FORMATS))
    depth_cap: int = 3
    include_analysis: bool = False

    @property
    def format_codes(self) -> list[str]:
        codes: list[str] = []
        for name in self.formats:
            code = FORMAT_CODES.get(name)
            if code is None:
                raise ValueError(f"unsupported format {name!r}")
            if code not in codes:
                codes.append(code)
        return codes

    def to_dict(self) -> dict[str, Any]:
        return {
            "project_name": self.project_name,
            "output_dir": self.output_dir,
            "formats": list(self.formats),
            "depth_cap": self.depth_cap,
            "include_analysis": self.include_analysis,
        }


def load_config(repo_root: Path | str) -> Config:
    """Load `codeatlas.json` from `repo_root` (falling back to defaults)."""
    root = Path(repo_root)
    path = root / CONFIG_FILENAME
    if not path.is_file():
        return Config(project_name=root.resolve().name)
    data = json.loads(path.read_text(encoding="utf-8"))
    return _from_dict(data, default_name=root.resolve().name)


def _from_dict(data: dict[str, Any], *, default_name: str) -> Config:
    return Config(
        project_name=data.get("project_name") or default_name,
        output_dir=data.get("output_dir", DEFAULT_OUTPUT_DIR),
        formats=list(data.get("formats", DEFAULT_FORMATS)),
        depth_cap=int(data.get("depth_cap", 3)),
        include_analysis=bool(data.get("include_analysis", False)),
    )


def init_config(
    repo_root: Path | str,
    *,
    force: bool = False,
    project_name: str | None = None,
    output_dir: str | None = None,
    formats: list[str] | None = None,
    depth_cap: int | None = None,
) -> Path:
    """Write a `codeatlas.json` opt-in file, returning its path."""
    root = Path(repo_root).resolve()
    path = root / CONFIG_FILENAME
    if path.exists() and not force:
        raise FileExistsError(f"{path} already exists (use --force to overwrite)")
    config = Config(
        project_name=project_name or root.name,
        output_dir=output_dir or DEFAULT_OUTPUT_DIR,
        formats=list(formats) if formats else list(DEFAULT_FORMATS),
        depth_cap=depth_cap if depth_cap is not None else 3,
    )
    # Validate format names early.
    _ = config.format_codes
    path.write_text(json.dumps(config.to_dict(), indent=2) + "\n", encoding="utf-8")
    return path
