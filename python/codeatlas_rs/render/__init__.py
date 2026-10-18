"""Documentation rendering for CodeAtlas `analysis.json`.

Renders the root overview plus one document per expanded component into
Markdown (Mermaid), HTML (Cytoscape.js), MDX, or reStructuredText. A render
manifest (`.codeatlas-render.json`) tracks owned files so stale docs are
removed and unowned files are never overwritten.
"""

from __future__ import annotations

import json
import os
import re
import tempfile
from pathlib import Path
from typing import Any

RENDER_MANIFEST_FILENAME = ".codeatlas-render.json"
RENDER_MANIFEST_VERSION = 1
ROOT_DOCUMENT_NAME = "overview"

FORMAT_EXTENSIONS = {"md": ".md", "html": ".html", "mdx": ".mdx", "rst": ".rst"}
SUPPORTED_FORMATS = tuple(FORMAT_EXTENSIONS)


def sanitize(name: str) -> str:
    return re.sub(r"\W+", "_", name)


def render(output_format: str, analysis_path: Path, output_dir: Path) -> list[Path]:
    """Render `analysis_path` in `output_format` into `output_dir`.

    Returns the list of files written. Raises `ValueError` on bad format or
    filename collisions, and `FileExistsError` when it would overwrite an
    unowned file.
    """
    if output_format not in SUPPORTED_FORMATS:
        raise ValueError(f"unsupported format {output_format!r}")

    data = _load_analysis(analysis_path)
    docs = _build_docs(data)
    extension = FORMAT_EXTENSIONS[output_format]

    writer = {
        "md": _render_markdown,
        "html": _render_html,
        "mdx": _render_mdx,
        "rst": _render_rst,
    }[output_format]

    generated: dict[str, str] = {}
    for key, doc in docs.items():
        name = ROOT_DOCUMENT_NAME if key == "__root__" else _doc_filename(key, doc)
        generated[f"{name}{extension}"] = writer(doc, data.get("metadata", {}).get("repo_name", ""))

    output_dir.mkdir(parents=True, exist_ok=True)

    previous = _load_manifest(output_dir)
    previous_names = set(previous.get("outputs", {}).get(output_format, []))

    for filename in generated:
        if filename in previous_names:
            continue
        if (output_dir / filename).exists():
            raise FileExistsError(
                f"refusing to overwrite unowned file {output_dir / filename}"
            )

    written: list[Path] = []
    with tempfile.TemporaryDirectory(prefix=".render-", dir=output_dir) as staging:
        staging_dir = Path(staging)
        for filename, content in generated.items():
            (staging_dir / filename).write_text(content, encoding="utf-8")
        for filename, content in generated.items():
            os.replace(staging_dir / filename, output_dir / filename)
            written.append(output_dir / filename)

    stale = previous_names - set(generated)
    for filename in stale:
        path = output_dir / filename
        if path.exists():
            path.unlink()

    _write_manifest(output_dir, output_format, sorted(generated))
    return sorted(written)


def _load_analysis(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _build_docs(data: dict[str, Any]) -> dict[str, Any]:
    docs: dict[str, Any] = {
        "__root__": {
            "components": data.get("components", []),
            "relations": data.get("components_relations", []),
            "description": data.get("description", ""),
        }
    }

    def collect(components: list[dict[str, Any]]) -> None:
        for component in components:
            nested = component.get("components")
            if nested:
                docs[_doc_key(component)] = {
                    "components": nested,
                    "relations": component.get("components_relations", []),
                    "description": component.get("description", ""),
                }
                collect(nested)

    collect(data.get("components", []))
    return docs


def _doc_key(component: dict[str, Any]) -> str:
    name = sanitize(component.get("name", "component"))
    component_id = component.get("component_id", "")
    return f"{name}.{component_id}" if component_id else name


def _doc_filename(key: str, doc: dict[str, Any]) -> str:
    return key


def _load_manifest(output_dir: Path) -> dict[str, Any]:
    path = output_dir / RENDER_MANIFEST_FILENAME
    if not path.exists():
        return {}
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return {}
    if manifest.get("version") != RENDER_MANIFEST_VERSION:
        return {}
    return manifest


def _write_manifest(output_dir: Path, output_format: str, names: list[str]) -> None:
    manifest = _load_manifest(output_dir)
    outputs = dict(manifest.get("outputs", {}))
    outputs[output_format] = names
    manifest = {
        "version": RENDER_MANIFEST_VERSION,
        "outputs": outputs,
    }
    path = output_dir / RENDER_MANIFEST_FILENAME
    tmp = output_dir / f".{RENDER_MANIFEST_FILENAME}.tmp"
    tmp.write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    os.replace(tmp, path)


# Imported lazily to avoid import cycles between render submodules.
def _render_markdown(doc: dict[str, Any], repo_name: str) -> str:
    from codeatlas_rs.render.markdown import generate_markdown

    return generate_markdown(doc)


def _render_html(doc: dict[str, Any], repo_name: str) -> str:
    from codeatlas_rs.render.html import generate_html

    return generate_html(doc)


def _render_mdx(doc: dict[str, Any], repo_name: str) -> str:
    from codeatlas_rs.render.mdx import generate_mdx

    return generate_mdx(doc)


def _render_rst(doc: dict[str, Any], repo_name: str) -> str:
    from codeatlas_rs.render.rst import generate_rst

    return generate_rst(doc)
