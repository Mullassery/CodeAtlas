"""CodeAtlas: Rust-native architecture maps for codebases."""

from __future__ import annotations

from codeatlas_rs import _core

__all__ = ["__version__", "analyze", "analyze_with_call_graph", "_core"]

__version__ = _core.version()


def analyze(
    repo_path: str,
    project_name: str | None = None,
    depth_cap: int = 3,
    ignore_paths: list[str] | None = None,
) -> tuple[str, str, str]:
    """Run a full deterministic analysis.

    `ignore_paths` lists extra directories (absolute or repo-relative) to exclude
    from discovery, e.g. a generated docs directory.

    Returns `(analysis_json, fingerprint_json, file_coverage_json)`.
    """
    return _core.analyze_repo(repo_path, project_name, depth_cap, ignore_paths)

def analyze_with_call_graph(
    repo_path: str,
    project_name: str | None = None,
    depth_cap: int = 3,
    ignore_paths: list[str] | None = None,
) -> tuple[str, str, str, str]:
    """Like :func:`analyze`, but also returns the merged call-graph JSON.

    Returns `(analysis_json, fingerprint_json, file_coverage_json,
    call_graph_json)`. The call graph is deliberately a sidecar so the
    established `analysis.json` schema is unchanged.
    """
    return _core.analyze_repo_full(repo_path, project_name, depth_cap, ignore_paths)
