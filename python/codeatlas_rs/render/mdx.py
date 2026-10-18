"""MDX rendering (Mermaid + frontmatter)."""

from __future__ import annotations

from typing import Any

from codeatlas_rs.render.markdown import mermaid


def generate_mdx(doc: dict[str, Any]) -> str:
    components = doc.get("components", [])
    relations = doc.get("relations", [])
    description = doc.get("description", "")

    lines = ["---", "title: CodeAtlas", "---", "", mermaid(components, relations), ""]
    if description:
        lines += [description, ""]
    for component in components:
        lines.append(f"# {component.get('name', '')}")
        lines.append("")
        lines.append(component.get("description", ""))
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"
