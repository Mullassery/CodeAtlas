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

from codeatlas_rs import __version__, analyze

ANALYSIS_FILENAME = "analysis.json"
FINGERPRINT_FILENAME = "fingerprint.json"
FILE_COVERAGE_FILENAME = "file_coverage.json"
OUTPUT_DIR_NAME = ".codeatlas"


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
    full.add_argument(
        "--cache",
        action="store_true",
        help="Skip analysis when repository contents are unchanged (content-addressed cache)",
    )

    render = sub.add_parser("render", help="Render an existing analysis.json")
    render.add_argument("analysis_path", type=str)
    render.add_argument("--format", choices=("md", "html", "mdx", "rst"), default="md")
    render.add_argument("--output-dir", type=str)

    diff = sub.add_parser("diff", help="Compare two analysis.json files")
    diff.add_argument("before", type=str)
    diff.add_argument("after", type=str)
    diff.add_argument("--format", choices=("text", "md", "json"), default="text")
    diff.add_argument(
        "--fail-on-change",
        action="store_true",
        help="Exit 1 when any architecture/method change is detected (for CI).",
    )

    return parser


def main(argv: list[str] | None = None) -> None:
    args = build_parser().parse_args(argv)
    if args.command == "render":
        render_main(
            _analysis=args.analysis_path, _format=args.format, _output_dir=args.output_dir
        )
        return
    if args.command == "diff":
        diff_main(
            before=args.before,
            after=args.after,
            output_format=args.format,
            fail_on_change=args.fail_on_change,
        )
        return
    run_full(args)


def diff_main(*, before: str, after: str, output_format: str, fail_on_change: bool) -> None:
    from codeatlas_rs.diff import (
        diff_analyses,
        load_analysis,
        render_markdown,
        render_text,
    )

    before_analysis = load_analysis(before)
    after_analysis = load_analysis(after)
    result = diff_analyses(before_analysis, after_analysis)

    if output_format == "json":
        print(json.dumps(result.to_dict(), indent=2))
    elif output_format == "md":
        print(render_markdown(result))
    else:
        print(render_text(result))

    if fail_on_change and result.has_changes:
        raise SystemExit(1)


def run_full(args: argparse.Namespace) -> None:
    repo_path = Path(args.local).resolve()
    if not repo_path.is_dir():
        raise SystemExit(f"codeatlas: not a directory: {args.local}")

    output_dir = Path(args.output_dir) if args.output_dir else repo_path / OUTPUT_DIR_NAME
    output_dir.mkdir(parents=True, exist_ok=True)

    cache = None
    cache_key = None
    if args.cache:
        from codeatlas_rs.cache import AnalysisCache, repo_fingerprint

        cache = AnalysisCache(output_dir / "cache")
        cache_key = repo_fingerprint(repo_path)
        cached = cache.load(cache_key)
        if cached is not None:
            _atomic_write(output_dir / ANALYSIS_FILENAME, cached.analysis_json)
            _atomic_write(output_dir / FINGERPRINT_FILENAME, cached.fingerprint_json)
            _atomic_write(output_dir / FILE_COVERAGE_FILENAME, cached.file_coverage_json)
            print(f"Analysis unchanged; reused cache for {cache_key[:12]}")
            if args.render:
                render_main(_analysis=str(output_dir / ANALYSIS_FILENAME), _format=args.render)
            return

    analysis_json, fingerprint_json, file_coverage_json = analyze(
        str(repo_path), project_name=args.project_name, depth_cap=args.depth_cap
    )

    analysis = json.loads(analysis_json)
    try:
        _maybe_enrich(analysis)
    except SystemExit:
        raise
    analysis_json = json.dumps(analysis, indent=2)

    _atomic_write(output_dir / ANALYSIS_FILENAME, analysis_json)
    _atomic_write(output_dir / FINGERPRINT_FILENAME, fingerprint_json)
    _atomic_write(output_dir / FILE_COVERAGE_FILENAME, file_coverage_json)
    if cache is not None and cache_key is not None:
        from codeatlas_rs.cache import CacheEntry

        cache.store(CacheEntry(cache_key, analysis_json, fingerprint_json, file_coverage_json))

    print(f"Analysis written to {output_dir / ANALYSIS_FILENAME}")

    if args.render:
        render_main(_analysis=str(output_dir / ANALYSIS_FILENAME), _format=args.render)


def _maybe_enrich(analysis: dict) -> None:
    from codeatlas_rs.llm import LLMError, enrich, resolve_config

    if resolve_config() is None:
        return
    try:
        enrich(analysis)
    except LLMError as exc:
        if exc.kind == "auth":
            print(f"\ncodeatlas: {exc}", file=sys.stderr)
            raise SystemExit(2) from exc
        if exc.kind == "quota":
            print(f"\ncodeatlas: {exc}", file=sys.stderr)
            raise SystemExit(3) from exc
        print(f"codeatlas: LLM enrichment failed, keeping deterministic names: {exc}", file=sys.stderr)


def render_main(
    _analysis: str | None = None, _format: str = "md", _output_dir: str | None = None
) -> None:
    if _analysis is None:
        parser = argparse.ArgumentParser(prog="codeatlas-render")
        parser.add_argument("analysis_path")
        parser.add_argument("--format", default="md", choices=("md", "html", "mdx", "rst"))
        parser.add_argument("--output-dir")
        args = parser.parse_args()
        _analysis = args.analysis_path
        _format = args.format
        _output_dir = args.output_dir

    analysis = Path(_analysis)
    if not analysis.is_file():
        raise SystemExit(f"codeatlas-render: analysis file not found: {_analysis}")

    output_dir = Path(_output_dir) if _output_dir else analysis.parent

    from codeatlas_rs.render import render

    written = render(_format, analysis, output_dir)
    for path in written:
        print(f"Rendered {path}")


def _atomic_write(path: Path, content: str) -> None:
    tmp = path.with_name(f".{path.name}.tmp")
    tmp.write_text(content, encoding="utf-8")
    os.replace(tmp, path)


if __name__ == "__main__":
    main()