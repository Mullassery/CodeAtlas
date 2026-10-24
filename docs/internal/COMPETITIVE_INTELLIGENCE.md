# Competitive intelligence in roadmap and upgrade work

**Policy date:** 2026-10-19

Competitive findings are an input to product decisions, not a one-off report.
Every pending roadmap item, new feature, and routine dependency upgrade should
check the repository's [`AUDIT_FINDINGS.md`](AUDIT_FINDINGS.md) before work
starts, and record the decision in the PR or changelog.

## Required check before building

1. **Read `AUDIT_FINDINGS.md`.** Find the relevant competitor row and the
   ranked next actions. If the finding is stale, update it with a dated source.
2. **Name the competitor and the user problem.** A feature is not justified by
   "competitor has it". State the workflow it unlocks, who needs it, and how
   often. Mark unverified demand as a hypothesis.
3. **Prefer reuse over new code.** Check whether an existing Mullassery repo
   already provides the capability (see `BUNDLES.md` in the profile repo and
   the bundle role file in this repo, when present). Typical reuse:
   - crawling/fetching/robots/SSRF → **MudFish**
   - architecture maps / PR diffs → **CodeAtlas**
   - data-quality contracts / validation → **StatGuardian**
   - token counting / cost tables → **PyTokenCalc**
   - dependency graph / advisories / SBOM → **PyDeepMesh**
   - segmentation / CLV / churn → **ClusterAudienceKit**
   - activation / lineage / PII → **PyReverseETL**
   - fleet state sync → **SyncFleet** (only after conflict resolution)
4. **Choose one disposition:** build, integrate, defer, or reject. Write it
   down. A rejection with a reason is a successful outcome.
5. **For routine upgrades**, check whether the upgrade closes a known
   competitor gap or breaks a differentiated capability. Run the tests that
   cover the affected path; do not merge a green build that silently changes a
   user-visible contract.

## PR / changelog checklist

- [ ] Competitor and source URL/date recorded (or "no direct competitor").
- [ ] User impact and frequency stated; unverified demand labelled.
- [ ] Reuse check done against sibling repos/bundles.
- [ ] Failure mode and rollback considered (especially auth, PII, cost).
- [ ] Docs/roadmap status updated honestly.
- [ ] Tests or a manual verification command included.

## Review cadence

- Refresh the competitor row when a release changes the capability.
- Re-check the full `AUDIT_FINDINGS.md` at least quarterly and before a major
  release.
- If a finding is wrong or stale, correct it in the same PR that relies on it.

## Non-goals

- Copying features for parity.
- Adding a dependency just because a competitor uses it.
- Treating star counts or download counts as proof of user need.

## OSS gap analysis — 2026-10-24

**What matters most here:** architecture rules, not just maps. dependency-cruiser can fail a build on a disallowed import; CodeAtlas should let teams declare boundary rules and enforce them, with type-aware resolution behind it.

Scope: CodeAtlas 0.8.2 (Rust core + PyO3 + Python wrapper) against OSS dependency-graph and architecture-analysis tools, judged on what teams use to map, document, and gate codebase architecture. "No" means not found in the reviewed public docs, not proof of absence. Sourcetrail (last release 2021, last commit 2021-12-13) was excluded as inactive; Sourcegraph was excluded because the reviewed repo is not the OSS product being compared; Mermaid is a rendering target, not an analyzer.

### Comparable OSS projects

| Project | GitHub | License | Last activity | Why it is the comparison |
|---|---|---|---|---|
| dependency-cruiser | https://github.com/sverweij/dependency-cruiser | MIT | v18.7.0 released 2026-10-19 (v18.8.0-beta-1 on 2026-10-23); last commit 2026-10-19 | Validates JS/TS dependency rules and emits graphs. The closest OSS competitor for architecture-boundary enforcement in CI. |
| madge | https://github.com/pahen/madge | MIT | v8.0.0 released 2024-08-05; last commit 2024-08-05 | Module dependency graphs, circular-dependency detection, and image output for JS/TS. Chosen for quick dependency visualization. |
| pydeps | https://github.com/thebjorn/pydeps | BSD-2-Clause | v3.0.9 released 2026-10-01; last commit 2026-10-01 | Python module dependency visualization with cycles, clustering, and Graphviz output. The Python-only equivalent of our import/call analysis. |

### Capability comparison

| Capability | Ours | dependency-cruiser | madge | pydeps | Gap |
|---|---|---|---|---|---|
| Multi-language extraction | Yes | Partial | Partial | Partial | Ours is ahead: 7 languages from one engine; competitors are JS/TS or Python only. |
| Symbol-level call graph | Yes | No | No | No | Ours is ahead; competitors stop at module imports. |
| Architecture component clustering | Yes | No | No | Partial | Ours is ahead with deterministic components; pydeps clustering is visual grouping only. |
| Deterministic output plus fingerprint | Yes | Partial | No | No | Ours is ahead; fingerprints and `file_coverage.json` are first-class. |
| Incremental analysis | Partial | No | No | No | Ours is weaker than per-file incrementality: `--cache` is whole-repository only. |
| Optional LLM naming/description | Yes | No | No | No | Ours is ahead; competitors generate no semantic component docs. |
| Rule-based architecture enforcement | No | Yes | Partial | No | Absent; dependency-cruiser validates user rules and reports violations. |
| PR architecture diff and gate | Yes | Partial | No | No | Ours is ahead on a dedicated `diff` and composite action. |
| Module alias/tsconfig resolution | No | Yes | Yes | Partial | Weaker; dependency-cruiser and madge resolve aliases through config. |
| Cycle detection | Partial | Yes | Yes | Yes | Weaker; import relationships exist but cycle reporting is not a documented CLI feature. |
| Output formats | Yes | Yes | Yes | Yes | Parity: md/Mermaid/HTML/MDX/rst versus dot/Mermaid/JSON/CSV/HTML and SVG/PNG. |
| LSP/type-aware resolution | No | No | No | Partial | Weaker; tree-sitter heuristics, no virtual dispatch or type hierarchy. |
| CI exit-code integration | Partial | Yes | Yes | No | Weaker: `--fail-on-change` exists, but no rule-level reporting. |
| Python 3.9+ install without Node | Yes | No | No | Yes | Ours and pydeps are pip-native; JS tools require Node. |

### Gaps that matter (ranked)

1. **No architecture rule engine.** Evidence: dependency-cruiser "validates [dependencies] against (your own) rules" and reports violations with a failing build; CodeAtlas only offers `diff --fail-on-change` on any change. Hurts platform teams that want "no imports from `ui` into `data`" rather than "nothing may change". Effort: M.
2. **Resolution fidelity is heuristic, not type-aware.** Evidence: `docs/audit/KNOWN_GAPS.md` G1 says tree-sitter resolution is "deliberately simpler than an LSP pipeline"; `docs/FEATURE_PARITY.md` lists LSP adapters and full cross-file resolution as later. dependency-cruiser and madge resolve tsconfig/webpack aliases; pydeps uses Python import machinery. Hurts monorepos with aliases, virtual dispatch, and generated code. Effort: L.
3. **Incremental analysis is only a whole-repo cache.** Evidence: README says `--cache` "is a whole-repository workflow cache, not per-file re-analysis"; FEATURE_PARITY defers CLI incremental/partial. Hurts large repositories where a one-file edit currently misses the cache. Effort: M.
4. **Kotlin and deep language parity remain absent.** Evidence: README lists Kotlin as blocked on tree-sitter grammar compatibility; AUDIT_FINDINGS notes generic types, nested classes, method references, and `super.method()` are unresolved. Hurts Android/Kotlin and type-heavy Java/C# teams comparing against IDE-grade tools. Effort: L.
5. **PyPI distribution is still macOS arm64 plus sdist for the shipped version.** Evidence: README says the release workflow builds manylinux/musllinux/Windows but PyPI 0.7.0 had only macOS arm64 plus sdist; QUEUE_STATUS confirms. Hurts Linux CI adoption where Node-free pip install is the selling point. Effort: S.

### Where we are differentiated

- One deterministic Rust engine produces symbols, call/reference edges, components, and rendered architecture docs across Python, Rust, Go, JS/TS, Java, PHP, and C#; the comparables are single-ecosystem module graphers.
- `analysis.json` plus a `call_graph.json` sidecar and `codeatlas query` give agents and editors symbol-level callers/callees without re-analysis or an LLM.
- Optional LLM naming/description runs only when configured; without a provider the pipeline still succeeds with deterministic names, unlike upstream-style fail-without-LLM behavior.
- PR architecture diff (`codeatlas diff`, `--fail-on-change`, composite action) turns drift into a reviewable artifact.
- Rendered output spans Markdown, Mermaid, HTML/Cytoscape, MDX, and reStructuredText, with a manifest that refuses to overwrite unowned files.
- No telemetry and a pure-Rust core with no network access; dependency-cruiser and madge are Node-based, pydeps is Python-only.

### Disposition

- **Build:** a declarative architecture-rule layer over the existing graph, per-file incremental re-analysis, and a cycle-reporting CLI. These close gaps without changing the analysis model.
- **Integrate/reuse:** keep Mermaid as a rendering target; borrow rule semantics and JSON report shapes from dependency-cruiser rather than inventing a format; use the existing PR diff action as the gate.
- **Defer/reject:** LSP-grade resolution and Kotlin stay deferred until a concrete consumer needs them; do not add a chat UI, hosted upload, or default telemetry (already rejected in FEATURE_PARITY).

### Sources

- https://github.com/sverweij/dependency-cruiser (accessed 2026-10-24)
- https://github.com/pahen/madge (accessed 2026-10-24)
- https://github.com/thebjorn/pydeps (accessed 2026-10-24)
- Repo files inspected: README.md, AUDIT_FINDINGS.md, COMPETITIVE_INTELLIGENCE.md, Cargo.toml, pyproject.toml, CHANGELOG.md, QUEUE_STATUS.md, docs/architecture/ARCHITECTURE.md, docs/FEATURE_PARITY.md, docs/IMPLEMENTATION_ROADMAP.md, docs/PR_DIFF.md, docs/CALL_GRAPH_QUERY.md, docs/audit/KNOWN_GAPS.md
