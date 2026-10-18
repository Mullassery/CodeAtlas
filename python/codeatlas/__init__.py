"""CodeAtlas: Rust-native architecture maps for codebases."""

from codeatlas import _core

__all__ = ["__version__", "analyze", "_core"]

__version__ = _core.version()


def analyze(
    repo_path: str, project_name: str | None = None, depth_cap: int = 3
) -> tuple[str, str, str]:
    """Run a full deterministic analysis.

    Returns `(analysis_json, fingerprint_json, file_coverage_json)`.
    """
    return _core.analyze_repo(repo_path, project_name, depth_cap)