"""Compare two CodeAtlas ``analysis.json`` files.

This is intentionally deterministic and dependency-free: it answers "what
changed in the architecture map" for a PR without re-running analysis.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class AnalysisDiff:
    added_components: tuple[str, ...] = ()
    removed_components: tuple[str, ...] = ()
    added_relations: tuple[str, ...] = ()
    removed_relations: tuple[str, ...] = ()
    added_methods: tuple[str, ...] = ()
    removed_methods: tuple[str, ...] = ()
    changed_methods: tuple[str, ...] = ()

    @property
    def has_changes(self) -> bool:
        return bool(
            self.added_components
            or self.removed_components
            or self.added_relations
            or self.removed_relations
            or self.added_methods
            or self.removed_methods
            or self.changed_methods
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "has_changes": self.has_changes,
            "added_components": list(self.added_components),
            "removed_components": list(self.removed_components),
            "added_relations": list(self.added_relations),
            "removed_relations": list(self.removed_relations),
            "added_methods": list(self.added_methods),
            "removed_methods": list(self.removed_methods),
            "changed_methods": list(self.changed_methods),
        }


def _component_keys(analysis: dict[str, Any]) -> dict[str, str]:
    result: dict[str, str] = {}
    for component in analysis.get("components", []):
        name = component.get("name", "")
        cid = component.get("component_id", name)
        result[str(cid)] = str(name)
    return result


def _relation_keys(analysis: dict[str, Any]) -> set[str]:
    result: set[str] = set()
    for relation in analysis.get("components_relations", []):
        src = relation.get("src_name", "")
        dst = relation.get("dst_name", "")
        kind = relation.get("relation", "")
        result.add(f"{src} -> {dst} ({kind})")
    return result


def _methods(analysis: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        str(key): value
        for key, value in (analysis.get("methods_index") or {}).items()
        if isinstance(value, dict)
    }


def diff_analyses(before: dict[str, Any], after: dict[str, Any]) -> AnalysisDiff:
    before_components = _component_keys(before)
    after_components = _component_keys(after)
    before_relations = _relation_keys(before)
    after_relations = _relation_keys(after)
    before_methods = _methods(before)
    after_methods = _methods(after)

    changed_methods: list[str] = []
    for key in sorted(set(before_methods) & set(after_methods)):
        old = before_methods[key]
        new = after_methods[key]
        if (
            old.get("content_hash") != new.get("content_hash")
            or old.get("start_line") != new.get("start_line")
            or old.get("end_line") != new.get("end_line")
            or old.get("type") != new.get("type")
        ):
            changed_methods.append(key)

    return AnalysisDiff(
        added_components=tuple(sorted(after_components[c] for c in set(after_components) - set(before_components))),
        removed_components=tuple(sorted(before_components[c] for c in set(before_components) - set(after_components))),
        added_relations=tuple(sorted(after_relations - before_relations)),
        removed_relations=tuple(sorted(before_relations - after_relations)),
        added_methods=tuple(sorted(set(after_methods) - set(before_methods))),
        removed_methods=tuple(sorted(set(before_methods) - set(after_methods))),
        changed_methods=tuple(changed_methods),
    )


def load_analysis(path: str) -> dict[str, Any]:
    data = json.loads(Path(path).read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise TypeError("analysis must be a JSON object")
    return data


def render_text(diff: AnalysisDiff) -> str:
    if not diff.has_changes:
        return "No architecture changes."
    lines: list[str] = ["CodeAtlas architecture diff", "=" * 30, ""]

    def section(title: str, values: tuple[str, ...]) -> None:
        if not values:
            return
        lines.append(f"{title} ({len(values)}):")
        lines.extend(f"  - {value}" for value in values)
        lines.append("")

    section("Added components", diff.added_components)
    section("Removed components", diff.removed_components)
    section("Added relations", diff.added_relations)
    section("Removed relations", diff.removed_relations)
    section("Added methods", diff.added_methods)
    section("Removed methods", diff.removed_methods)
    section("Changed methods", diff.changed_methods)
    return "\n".join(lines).rstrip()


def render_markdown(diff: AnalysisDiff) -> str:
    if not diff.has_changes:
        return "No architecture changes."
    lines: list[str] = ["## CodeAtlas architecture diff", ""]

    def section(title: str, values: tuple[str, ...], code: bool = False) -> None:
        if not values:
            return
        lines.append(f"### {title} ({len(values)})")
        for value in values:
            lines.append(f"- `{value}`" if code else f"- {value}")
        lines.append("")

    section("Added components", diff.added_components)
    section("Removed components", diff.removed_components)
    section("Added relations", diff.added_relations)
    section("Removed relations", diff.removed_relations)
    section("Added methods", diff.added_methods, code=True)
    section("Removed methods", diff.removed_methods, code=True)
    section("Changed methods", diff.changed_methods, code=True)
    return "\n".join(lines).rstrip()
