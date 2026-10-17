"""reStructuredText rendering (Sphinx mermaid directive)."""

from __future__ import annotations

from typing import Any

from codeatlas.render import sanitize


def generate_rst(doc: dict[str, Any]) -> str:
    components = doc.get("components", [])
    relations = doc.get("relations", [])
    description = doc.get("description", "")

    lines = ["CodeAtlas", "=========", "", ".. mermaid::", "", "   graph LR"]
    for component in components:
        key = sanitize(component["name"])
        lines.append(f"   {key}[\"{component['name']}\"]")
    for relation in relations:
        src = sanitize(relation["src_name"])
        dst = sanitize(relation["dst_name"])
        lines.append(f"   {src} -- \"{relation['relation']}\" --> {dst}")
    lines.append("")

    if description:
        lines += [description, ""]

    for component in components:
        name = component.get("name", "")
        lines.append(name)
        lines.append("^" * len(name))
        lines.append("")
        lines.append(component.get("description", ""))
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"
