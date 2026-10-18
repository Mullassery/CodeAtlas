"""`codeatlas-bundle` command-line interface."""

from __future__ import annotations

import argparse
import sys

from codeatlas_bundle import __version__
from codeatlas_bundle.bundle import bundle, check, detect_repo_root
from codeatlas_bundle.config import init_config, load_config


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="codeatlas-bundle",
        description="Bundle CodeAtlas architecture docs into a repository",
    )
    parser.add_argument("--version", action="version", version=f"codeatlas-bundle {__version__}")
    sub = parser.add_subparsers(dest="command", required=True, metavar="COMMAND")

    init = sub.add_parser("init", help="Write a codeatlas.json opt-in file")
    init.add_argument("--repo", default=".", help="Repository root")
    init.add_argument("--project-name")
    init.add_argument("--output-dir")
    init.add_argument("--formats", help="Comma-separated: markdown,html,mdx,rst")
    init.add_argument("--depth-cap", type=int)
    init.add_argument("--force", action="store_true")

    build = sub.add_parser("build", help="Generate the bundle (and optionally commit)")
    build.add_argument("--repo", default=".", help="Repository root")
    build.add_argument("--commit", action="store_true", help="Commit the generated docs")
    build.add_argument("--message", help="Commit message")

    chk = sub.add_parser("check", help="Fail if the committed bundle is stale")
    chk.add_argument("--repo", default=".", help="Repository root")

    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "init":
        return _cmd_init(args)
    if args.command == "check":
        return _cmd_check(args)
    return _cmd_build(args)


def _cmd_init(args: argparse.Namespace) -> int:
    root = detect_repo_root(args.repo)
    formats = args.formats.split(",") if args.formats else None
    try:
        path = init_config(
            root,
            force=args.force,
            project_name=args.project_name,
            output_dir=args.output_dir,
            formats=formats,
            depth_cap=args.depth_cap,
        )
    except (FileExistsError, ValueError) as exc:
        print(f"codeatlas-bundle: {exc}", file=sys.stderr)
        return 1
    print(f"Wrote {path}")
    print("Next: codeatlas-bundle build --commit")
    return 0


def _cmd_build(args: argparse.Namespace) -> int:
    root = detect_repo_root(args.repo)
    result = bundle(root, commit=args.commit, message=args.message)
    for path in result.files:
        print(f"Wrote {path.relative_to(result.repo_root)}")
    if args.commit:
        state = "committed" if result.committed else "no changes to commit"
        print(f"Bundle {state}")
    return 0


def _cmd_check(args: argparse.Namespace) -> int:
    root = detect_repo_root(args.repo)
    current, changed = check(root)
    if current:
        print(f"Bundle is up to date ({load_config(root).output_dir})")
        return 0
    print("codeatlas-bundle: bundle is stale; run `codeatlas-bundle build`:")
    for rel in changed:
        print(f"  - {rel}")
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
