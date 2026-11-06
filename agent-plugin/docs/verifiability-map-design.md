# Crate Verifiability Map: design

Status: the vertical slice described under "Implemented" ships in this plugin. Everything under
"Proposed" is not implemented, and nothing in the plugin pretends otherwise.

## Goal

Give a coding agent (and its user) the verification structure of a Rust workspace: which
functions exist, who calls whom, which are recursive, which already have harnesses, contracts, and
Kani results, which are hard, and in which order to verify them. The map is a **planning aid, not
a proof certificate.**

Non-goals: predicting solver time or success (no static score can), replacing Kani's own
analysis, or turning partial results into claims about the whole crate.

## Single source of truth

The map is produced by scripts in this plugin (`scripts/map-crate.py`, `scripts/verifiability_map.py`),
versioned with Kani, and read by every agent through the same skill. Marketplace packages wrap this
directory; none forks it. It works without MCP.

## Implemented

| Piece | What it does |
|-------|--------------|
| Workspace discovery | `cargo metadata`: packages, targets (lib, bin, ...), features, workspace dependencies. Only library and binary sources are indexed; tests, examples, and benchmarks are not verification targets. |
| Function index | A text scan (the plugin's parser) finds every function with a body: stable ID, source location, visibility, generics, `unsafe`, `cfg` attributes, contracts (`requires`, `ensures`, `modifies`), and structural **signals**. |
| Call graph | From `cargo-callgraph` (rust-analyzer based; optional; invoked as an external program, never bundled, because it is AGPL-3.0-or-later) when installed, otherwise from a text scan. Every edge records `kind`, `resolved`, `source`, and call site. |
| Graph analysis | Callers and callees, strongly connected components (Tarjan), recursion (self and mutual), and levels (leaves are level 0) on the resolved edges. |
| Harness index | `#[kani::proof]` and `#[kani::proof_for_contract]` harnesses with unwind bounds, stubs, `stub_verified`, assumptions, and what each harness targets and **provably reaches**. |
| Kani results | `--export-json` files and evidence bundles, merged per harness with the exact configuration (Kani version, unwind, solver, bounded inputs, disabled checks, stubs) and a freshness check. |
| Status | Per function: `verified`, `conditional`, `failed`, `inconclusive`, `unsupported`, `stale`, `not_attempted`, with the evidence and caveats behind it. |
| Strategy and priority | A documented heuristic that proposes a Kani workflow for each function and an order. |
| Plan | A staged, bottom-up plan; goals restrict its scope. |
| Update | Merge new results into a map, recompute everything, and refuse to run when the sources changed since the map was built. |
| Report | A Mermaid diagram of the call graph, colored by status. |
| Schema | `schemas/verifiability-map.schema.json`, versioned (`schema_version`). Output is deterministic: same sources and results give the same bytes. |

## Proposed (not implemented)

- `cargo kani complexity`: the five per-function metrics (`branches`, `loops`, `function_calls`,
  `arithmetic_operations`, `memory_operations`) computed from MIR by `kani-compiler`, as a separate
  Kani feature. The map reserves `complexity` in every function record, with status `not_computed`
  and those five metric names set to `null`. The plugin will fill the field from the command's JSON
  output, joined by function identifier, and will not define or estimate those metrics itself.
- A compiler-based call graph: `tools/scanner` in this repository already builds a call graph from
  MIR (`rustc_public`) and finds direct recursion and loops, but writes CSV and misses mutual
  recursion; extending it would give monomorphized, exact direct calls and replace the
  rust-analyzer approximation.
- MCP tools (`kani_map_crate`, `kani_plan_verification`, `kani_update_verification_map`; names are
  proposals). Until then agents run the CLI.
- Monomorphized instances, per-target and per-feature maps (several maps, one per configuration),
  a measured-cost model (recording solver time and memory per harness and studying how they relate to
  the complexity metrics), and an HTML report.

## Data sources and how far to trust them

| Source | Gives | Cannot give |
|--------|-------|-------------|
| `cargo metadata` | Exact workspace structure | Anything inside functions |
| `cargo-callgraph` | Resolved direct calls, trait-level edges | Exact dynamic dispatch targets, closures, macro-generated calls, a feature set (it uses its default configuration) |
| Text scan | Function structure, contracts, harnesses, signals; a fallback call graph by name | `#[cfg]` evaluation, types (method calls resolve by name only), macro expansion |
| rust-analyzer (via the skill) | Navigation and references while writing harnesses | An exact call graph: its answers are approximations |
| Kani results | What was actually checked, under which configuration | Anything outside the harness's inputs and bounds |
| Miri | Concrete UB evidence on test inputs | Proofs; its findings are never reported as Kani results |

## Identity, provenance, determinism

- **Function ID**: `crate::module::path::name`, with `<Type as Trait>` for trait impls and `Type`
  for inherent impls; collisions are disambiguated by file and then by order of appearance. IDs do
  not contain line numbers, so they survive edits that move code.
- **Provenance**: every call edge says which tool produced it and how it was resolved; every
  evidence item names the result file, Kani version, and freshness; the map names its tools and the
  call-graph source. Uncertainty is a list of `{flag, reason}` entries, globally and per function.
- **Determinism**: keys are sorted, lists are ordered by ID, paths are relative to the workspace
  root, and no timestamps are written.

## Status rules

A harness result counts toward a function only if the function is the harness's contract target
or is **provably reached**: reachable through resolved call edges, not through stubs, and not only
through a name-matched or trait-level edge. Evidence is *fresh* if the evidence bundle's source
hashes still match (or, for a bare export, no source was modified after the run started).

| Status | Meaning |
|--------|---------|
| `verified` | At least one fresh harness that provably reaches the function succeeded, and no fresh one failed, timed out, or hit an unsupported construct. The scope is the harness: its inputs, assumptions, bounds, stubs, and disabled checks, listed as caveats. Never a claim about the crate. |
| `conditional` | Verified, but the harness stubs a function or assumes a function's contract (`stub_verified`) that nothing verifies. The condition is named. |
| `failed` | A fresh harness that reaches it failed. This may be a bug in the code or a missing precondition in the harness. |
| `inconclusive` | A fresh run timed out, ran out of memory, or crashed. |
| `unsupported` | Kani reported unsupported constructs. |
| `stale` | Evidence exists, but the sources changed after it was produced. |
| `not_attempted` | No harness provably reaches it. |

Precedence over fresh evidence: failed, inconclusive, unsupported, conditional, verified. An
unverified callee is never treated as proven: a function's `unverified_callees` and `blocked_by`
list the resolved callees that are not `verified`.

## Strategy and priority (heuristics)

Strategy picks, in order: the response to an existing result (triage a failure, reduce an
inconclusive run, re-run stale evidence, avoid an unsupported construct), then the first structural
obstacle (recursion: contracts for every member of the component; loops: unwind bound or loop
invariant; generics: concrete instances; unresolved or trait-level callees: stubs or contracts;
unbounded input types: bounded inputs; `unsafe`: UB-focused checks), then an existing harness, then
AutoHarness for obstacle-free leaves.

`priority = impact / effort`, where `impact = 1 + transitive callers + 2 if public` and
`effort = 1 + 2 * loop keywords + 0.5 * branch keywords + 0.5 * callees + 3 if recursive + 2 * unresolved
callees + 1 if generic + 1 per unbounded input type`. These weights are judgment, not measurement:
they order work, they do not predict solver time or whether a proof will succeed.

## Limitations

Missing call edges, dynamic dispatch, closures, macros, FFI, `unsafe` code, `cfg` differences, and
generic instances are represented as uncertainty, never ignored. The text-scan fallback does not
evaluate `#[cfg]`, so code for all features is indexed. A map describes one point in time; use
`update` (which checks source hashes) or re-map after edits.

## Validation

Unit tests (`tests/test_map.py`) cover loops, direct and mutual recursion, generics, trait dispatch,
macros, features, multiple crates and targets, contracts, failed, inconclusive, unsupported,
stale, and conditional results, determinism, the schema, planning, goals, and update. The smoke test
(`tests/agent-plugin/verifiability-map`) runs real Kani on a two-crate workspace and builds the map
with both call-graph sources when `cargo-callgraph` is installed.

Measured on this repository's own workspace (15 packages, 2,887 functions, 231 source files):

| Call graph | Time | Map size | Resolved edges | Unresolved edges | Recursive components |
|------------|------|----------|----------------|------------------|----------------------|
| Text scan | 10 s | 16 MB | 2,034 | 27,623 | 35 |
| `cargo-callgraph` | 28 s | 8 MB | 3,897 | 5,165 | 45 |

The first version of the text scan took 187 s and produced a 291 MB map on this workspace: common
method names (`new`, `len`) matched hundreds of workspace methods, and the root package scanned the
whole repository. Method names with more than five candidates are now one external edge, and only
library and binary source directories are scanned. The tool finds more calls and fewer spurious
candidates; use it when it is installed.
