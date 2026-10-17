# Feature Parity

Disposition of every upstream feature from `docs/audit/FEATURE_INVENTORY.md`:
**v1** implement in the initial release, **later** implement post-v1, **exclude** with justification,
**blocked** pending investigation.

## Implement in v1
| Feature (upstream) | CodeAtlas deliverable | Notes |
|---|---|---|
| CLI-01 `full` (local) | `codeatlas --local` | Rust core + wrapper |
| CLI-02 `render`-only | `codeatlas-render` | wrapper |
| CLI-06 `--render` md/html/mdx/rst | rendering | wrapper |
| CLI-07 `--depth-cap` | same flag, metadata contract | core+wrapper |
| SA-02 repo ignore | Rust gitwildmatch ignore | core |
| SA-01 language detection | tree-sitter suffix detection (no tokei) | core (drop binary dep) |
| SA-12/14 call graph + package deps | tree-sitter call/import/package extraction | core (subset) |
| SA-13 class hierarchy | syntactic extends/implements/impl | core (subset) |
| SA-15/16 references + promotion | Rust symbol table + references | core |
| CL-01 deterministic partition | Rust directory-based hierarchical partition (simplified) | core (full draft/replay port deferred) |
| CL-02 groupers kinship/affinity | Rust (simplified: deterministic directory partition + single-child absorption) | core (full affinity fold deferred) |
| CL-03 TreeSpec v4 | serde `tree_spec` | core |
| CL-04/05/06 ids, absorption, expand | Rust | core |
| LLM-02/03 (naming/desc/relations) | wrapper via provider | wrapper (only when configured) |
| SR-01 analysis.json | serde structs, upstream-compatible | core |
| SR-02 fingerprint | sha256[:16] + tree hash | core |
| SR-03 file_coverage | same | core |
| SR-04..07 markdown/mermaid/html/mdx/rst | wrapper | wrapper |
| SR-08 render manifest | wrapper | wrapper |

## Implement later
| Feature | Reason to defer |
|---|---|
| CLI-02 `incremental`, CLI-03 `partial` | needs fingerprint diff + scoped re-cluster (depends on stable TreeSpec + LSP/graph fidelity) |
| SA-19 concurrency/memory budget | optimize after correctness |
| LSP adapters (typeHierarchy, virtual dispatch, full cross-file resolution) | large; tree-sitter first |
| Kotlin, C#, Java full-parity analysis | LSP-backed; heavy servers |
| SA-17 typed cache + warm-start | after stable analysis |
| LLM-04 incremental scope ops, LLM-05 planner grouper, LLM-07 context-window catalogs | wrapper, post-v1 |
| EC-04 health checks | deterministic; post-v1 |
| C# `.cs` full analysis | needs `dotnet`; post-v1 LSP |

## Exclude (with justification)
| Feature | Justification |
|---|---|
| CLI-10 remote clone + `--upload` | hosted-artifact coupling, obsolete naming |
| PostHog telemetry (default-on) | privacy; CodeAtlas ships no telemetry |
| `codeboarding-setup`/nodeenv/tool installer | non-essential to core value; document manual setup |
| C++ analysis | upstream declared-only (no adapter) |
| Web platform / VS Code / GitHub Action | external products, out of repo scope |
| Pickle cache (`static_analysis.pkl`) | unpickle = code-execution risk; use typed serialization instead |

## Blocked pending investigation
| Feature | Blocker |
|---|---|
| Exact byte-parity of `analysis.json` vs a fresh upstream run | no access to private integration fixtures; needs a shared fixture analyzed by both (G3) |
| Reproduce upstream cluster *names* deterministically | memberships are deterministic, names may be LLM/tie-order dependent (G9) |

## Compatibility contract
- **Output schema:** `analysis.json`, `fingerprint.json`, `file_coverage.json` field names/layout are
  preserved to keep downstream readers (web viewer, editors, docs tooling) working. No guarantee of
  byte-for-byte parity of *LLM-authored* text fields.
- **CLI:** CodeAtlas uses `codeatlas`/`codeatlas-render`; flags mirror upstream where sensible but the
  program name is distinct (independent tool, not a drop-in imposter).
- **Python API:** the wrapper mirrors upstream function/class names (`run_full`, `build_generator`,
  `DiagramGenerator`, `depth_cap`) as a familiarity affordance, documented as a separate API.