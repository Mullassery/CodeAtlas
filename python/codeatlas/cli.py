"""CodeAtlas command-line interface.

The deterministic analysis path is delegated to the Rust core (`codeatlas._core`);
this module wires arguments, configuration, rendering, and (later) optional LLM
enrichment. Milestone status: M0/M5 — see docs/IMPLEMENTATION_ROADMAP.md.
"""

from __future__ import annotations

import argparse

from codeatlas import __version__


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="codeatlas",
        description="Generate architecture maps for codebases",
    )
    parser.add_argument(
        "--version", action="version", version=f"codeatlas {__version__}"
    )
    sub = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")

    full = sub.add_parser("full", help="Run a full analysis on a local repository")
    full.add_argument("--local", type=str, help="Path to a local repository")
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
        render_main(_analysis=args.analysis_path, _format=args.format, _output_dir=args.output_dir)
    else:
        # M5 end-to-end full analysis lands here; M0 refuses loudly rather than
        # claiming an unimplemented feature.
        raise SystemExit(
            "codeatlas: the full analysis workflow is not implemented yet "
            "(milestone M5). See docs/IMPLEMENTATION_ROADMAP.md."
        )


def render_main(
    _analysis: str | None = None, _format: str = "md", _output_dir: str | None = None
) -> None:
    raise SystemExit(
        "codeatlas-render: rendering is not implemented yet "
        "(milestone M6). See docs/IMPLEMENTATION_ROADMAP.md."
    )


if __name__ == "__main__":
    main()