"""HTML (Cytoscape.js + dagre) rendering."""

from __future__ import annotations

import json
from typing import Any

from codeatlas.render import sanitize


def generate_html(doc: dict[str, Any]) -> str:
    components = doc.get("components", [])
    relations = doc.get("relations", [])
    description = doc.get("description", "")

    nodes = []
    for component in components:
        key = sanitize(component["name"])
        node = {
            "data": {
                "id": key,
                "label": component.get("name", ""),
                "description": component.get("description", ""),
            }
        }
        if component.get("can_expand"):
            node["data"]["linkUrl"] = f"./{key}.html"
        nodes.append(node)

    edges = []
    for i, relation in enumerate(relations):
        src = sanitize(relation["src_name"])
        dst = sanitize(relation["dst_name"])
        edges.append(
            {
                "data": {
                    "id": f"edge_{i}",
                    "source": src,
                    "target": dst,
                    "label": relation.get("relation", ""),
                }
            }
        )

    data = json.dumps({"nodes": nodes, "edges": edges})

    return f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>CodeAtlas</title>
<script src="https://unpkg.com/cytoscape@3.23.0/dist/cytoscape.min.js"></script>
<script src="https://unpkg.com/dagre@0.8.5/dist/dagre.min.js"></script>
<script src="https://unpkg.com/cytoscape-dagre@2.4.0/cytoscape-dagre.js"></script>
<style>
  body {{ margin: 0; font-family: -apple-system, sans-serif; }}
  #cy {{ width: 100vw; height: 100vh; }}
  header {{ padding: 12px 16px; border-bottom: 1px solid #eee; }}
</style>
</head>
<body>
<header>
  <strong>CodeAtlas</strong>
  {description and f'<p>{description}</p>'}
</header>
<div id="cy"></div>
<script>
const data = {data};
const cy = cytoscape({{
  container: document.getElementById('cy'),
  elements: data,
  style: [
    {{ selector: 'node', style: {{
      'label': 'data(label)',
      'shape': 'round-rectangle',
      'background-color': '#eef2ff',
      'border-color': '#6366f1',
      'border-width': 1,
      'width': 'label',
      'padding': '10px',
      'text-valign': 'center',
      'text-halign': 'center',
      'font-size': '12px',
    }}}},
    {{ selector: 'edge', style: {{
      'label': 'data(label)',
      'curve-style': 'bezier',
      'target-arrow-shape': 'triangle',
      'font-size': '10px',
      'text-rotation': 'autorotate',
    }}}}
  ],
  layout: {{ name: 'dagre', rankDir: 'LR' }}
}});
cy.on('tap', 'node', evt => {{
  const node = evt.target;
  if (node.data('linkUrl')) {{ window.location.href = node.data('linkUrl'); }}
}});
</script>
</body>
</html>
"""
