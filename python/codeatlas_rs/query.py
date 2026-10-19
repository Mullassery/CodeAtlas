"""Query a generated call graph for local agents and editors.

The Rust pipeline already extracts symbols and call/reference edges; this module
exposes them from ``.codeatlas/call_graph.json`` without re-running analysis.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


def load_call_graph(path_or_repo: str | Path) -> dict[str, Any]:
    """Load ``call_graph.json`` from a file or a repository's ``.codeatlas/`` dir."""
    path = Path(path_or_repo).expanduser()
    if path.is_dir():
        path = path / ".codeatlas" / "call_graph.json"
    if not path.is_file():
        raise FileNotFoundError(f"call graph not found: {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError("call graph must be a JSON object")
    return data


def _nodes(call_graph: dict[str, Any]) -> list[dict[str, Any]]:
    return [node for node in call_graph.get("nodes", []) if isinstance(node, dict)]


def _edges(call_graph: dict[str, Any]) -> list[dict[str, Any]]:
    return [edge for edge in call_graph.get("edges", []) if isinstance(edge, dict)]


def _symbol_matches(call_graph: dict[str, Any], name: str) -> list[dict[str, Any]]:
    nodes = _nodes(call_graph)
    exact = [node for node in nodes if node.get("fully_qualified_name") == name]
    if exact:
        return exact
    matches = []
    for node in nodes:
        qualified = str(node.get("fully_qualified_name", ""))
        if qualified.rsplit(".", 1)[-1] == name or qualified.endswith("." + name):
            matches.append(node)
    return matches


def callers(call_graph: dict[str, Any], name: str) -> list[dict[str, Any]]:
    """Edges whose destination is a matching symbol."""
    targets = {node.get("fully_qualified_name") for node in _symbol_matches(call_graph, name)}
    return [edge for edge in _edges(call_graph) if edge.get("destination") in targets]


def callees(call_graph: dict[str, Any], name: str) -> list[dict[str, Any]]:
    """Edges whose source is a matching symbol."""
    sources = {node.get("fully_qualified_name") for node in _symbol_matches(call_graph, name)}
    return [edge for edge in _edges(call_graph) if edge.get("source") in sources]


def describe(
    call_graph: dict[str, Any], name: str, *, direction: str = "both"
) -> dict[str, Any]:
    """Return matching symbols plus caller/callee edges (JSON-serializable)."""
    result: dict[str, Any] = {
        "query": name,
        "symbols": _symbol_matches(call_graph, name),
    }
    if direction in ("callers", "both"):
        result["callers"] = callers(call_graph, name)
    if direction in ("callees", "both"):
        result["callees"] = callees(call_graph, name)
    return result


def _first_line(edge: dict[str, Any]) -> int | None:
    sites = edge.get("call_sites") or []
    if sites and isinstance(sites[0], dict):
        return sites[0].get("line")
    return None


def query_main(
    path: str | Path,
    symbol: str,
    *,
    direction: str = "both",
    output_format: str = "text",
) -> None:
    """CLI entry point for ``codeatlas query``."""
    call_graph = load_call_graph(path)
    result = describe(call_graph, symbol, direction=direction)

    if output_format == "json":
        print(json.dumps(result, indent=2))
        return

    if not result["symbols"]:
        print(f"No symbol matching {symbol!r}")
        return

    for node in result["symbols"]:
        print(
            f"{node.get('fully_qualified_name')}  "
            f"{node.get('file_path')}:{node.get('line_start')}-{node.get('line_end')}  "
            f"{node.get('node_type')}"
        )
    for kind in ("callers", "callees"):
        if kind not in result:
            continue
        print(f"{kind}:")
        for edge in result[kind]:
            other = edge.get("source") if kind == "callers" else edge.get("destination")
            line = _first_line(edge)
            suffix = f" (line {line})" if line else ""
            print(f"  {other}{suffix}")
