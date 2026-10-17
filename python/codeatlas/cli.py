"""CodeAtlas command-line interface.

The deterministic analysis path is delegated to the Rust core (`codeatlas._core`);
this module wires arguments, output writing, and (later) rendering and optional
LLM enrichment.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

from codeatlas import __version__, analyze

ANALYSIS_FILENAME = "analysis.json"
FINGERPRINT_FILENAME = "fingerprint.json"
OUTPUT_DIR_NAME = ".codeboarding"


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="codeatlas",
        description="Generate architecture maps for codebases",
    )
    parser.add_argument("--version", action="version", version=f"codeatlas {__version__}")
    sub = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")

    full = sub.add_parser("full", help="Run a full analysis on a local repository")
    full.add_argument("--local", type=str, required=True, help="Path to a local repository")
    full.add_argument("--output-dir", type=str, help="Output directory")
    full.add_argument("--project-name", type=str, help="Project name")
    full.add_argument("--depth-cap", type=int, default=3)
    full.add_argument(
        "--render", choices=("md", "html", "mdx", "rst"), help="Render docs after analysis"
    )

    render = sub.add_parser("render", help="Render an existing analysis.json")
    render.add_argument("analysis_path", type=str)
    render.add_argument("--format", choices=("md", "html", "mdx", "rst"), default="md")
    render.add_argument("--output-dir", type=str)

    return parser


def main(argv: list[str] | None = None) -> None:
    args = build_parser().parse_args(argv)
    if args.command == "render":
        render_main(
            _analysis=args.analysis_path, _format=args.format, _output_dir=args.output_dir
        )
        return
    run_full(args)


def run_full(args: argparse.Namespace) -> None:
    repo_path = Path(args.local).resolve()
    if not repo_path.is_dir():
        raise SystemExit(f"codeatlas: not a directory: {args.local}")

    output_dir = Path(args.output_dir) if args.output_dir else repo_path / OUTPUT_DIR_NAME
    output_dir.mkdir(parents=True, exist_ok=True)

    analysis_json, fingerprint_json = analyze(
        str(repo_path), project_name=args.project_name, depth_cap=args.depth_cap
    )

    _atomic_write(output_dir / ANALYSIS_FILENAME, analysis_json)
    _atomic_write(output_dir / FINGERPRINT_FILENAME, fingerprint_json)

    print(f"Analysis written to {output_dir / ANALYSIS_FILENAME}")

    if args.render:
        render_main(_analysis=str(output_dir / ANALYSIS_FILENAME), _format=args.render)


def render_main(
    _analysis: str | None = None, _format: str = "md", _output_dir: str | None = None
) -> None:
    raise SystemExit(
        "codeatlas-render: rendering is not implemented yet "
        "(milestone M6). See docs/IMPLEMENTATION_ROADMAP.md."
    )


def _atomic_write(path: Path, content: str) -> None:
    tmp = path.with_name(f".{path.name}.tmp")
    tmp.write_text(content, encoding="utf-8")
    os.replace(tmp, path)


if __name__ == "__main__":
    main()