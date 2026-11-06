# Planning a verification with the Crate Verifiability Map

For more than one function, plan before writing harnesses. The map shows the call structure of a
workspace, what already has a harness and a result, what is recursive or otherwise hard, and in
which order to work. It is a **planning aid, not a proof**: read `Limits` below before trusting it.

```bash
scripts/map-crate.py map . --results kani.json --out map.json   # build (results optional)
scripts/map-crate.py plan map.json [--goal FUNCTION]...          # staged, bottom-up plan
scripts/map-crate.py explain map.json FUNCTION                   # one function in detail
scripts/map-crate.py update map.json --results new.json          # merge new results, re-plan
scripts/map-crate.py report map.json --out graph.mmd             # Mermaid diagram
```

`--results` takes `--export-json` files and evidence bundles (`evidence.py`). The call graph comes
from `cargo-callgraph` when it is installed (`cargo install cargo-callgraph`; an optional external
program, AGPL-3.0, never bundled) and from a text scan otherwise: the map's `configuration.call_graph`
says which. Prefer the tool: on a large workspace it finds about twice the calls with a fifth of the
spurious candidates.

## Workflow

1. Map the workspace. Read `uncertainty` first: it lists what the map cannot see.
2. Agree the goal with the user (a function, a module, a public API). Pass it as `--goal`; the plan
   is then limited to the goal and what it depends on.
3. Follow the plan bottom-up. Each stage's functions are ready once their `blocked_by` callees are
   verified or covered by a contract (`stub_verified`, `references/contracts.md`). Work a
   recursive component as one unit.
4. After each Kani run, `update` the map with the new export. If it refuses because the sources
   changed, map again. Re-plan: failed and inconclusive functions change the order and the strategy.
5. Report from the map's evidence, with its caveats. Never report a map as a proof.

## Reading a function

- `verification.status`: `verified`, `conditional`, `failed`, `inconclusive`, `unsupported`, `stale`,
  or `not_attempted`; the evidence behind it, and `caveats` (assumptions, bounds, stubs, disabled
  checks). `verified` means a *fresh* harness that *provably reaches* the function succeeded, under
  those caveats. `conditional` means the result depends on a stub or on a contract that nothing
  verifies: verify that first. `stale` means the sources changed after the result.
- `callees` / `callers`: resolved edges only. `candidate_callees` (name-matched, trait-level) and
  `unresolved_callees` (dispatch, unknown) are not resolved: a callee reached only through them is
  never counted as verified or covered.
- `component`, `recursive`, `level`: level 0 are leaves; recursive components need contracts for
  every member.
- `strategy.primary` and `rationale`: which Kani workflow to try first, and why.
- `priority`: impact / effort, a heuristic order. It does not predict solver time.
- `complexity`: reserved for the five metrics of the proposed `cargo kani complexity` command, and
  `not_computed` until it exists. `signals` are different: text-based counts (loop and branch
  keywords, `unsafe`, unbounded input types) that guide the heuristic.

## Strategies

| `primary` | Do |
|-----------|----|
| `triage-failure` | Replay the counterexample (`kani_get_counterexample`) and decide: bug, or missing precondition |
| `reduce-or-split` | The run was inconclusive: shrink bounds, stub expensive callees, split the harness (`references/kani.md`, performance) |
| `avoid-unsupported-construct` | Stub or restructure what Kani does not support |
| `re-run` | The evidence is stale: re-run the harness |
| `contract-and-recursion` | Write `requires`/`ensures` for every member of the recursive component and prove each (`references/contracts.md`) |
| `unwind-or-loop-contract` | Choose an unwind bound or a loop invariant |
| `instantiate-concrete-types` | Verify concrete instantiations of the generic |
| `stub-or-contract-callees` | Callees are trait-level, dynamic, or unknown: stub them or give them contracts |
| `bounded-inputs` | Bound slices, strings, `Vec`s (`bounded_any`, `--bounded-arguments`) |
| `ub-focused-harness` | Also use `-Z valid-value-checks` and `-Z uninit-checks` |
| `run-existing-harness` | A harness exists but has no result yet |
| `autoharness-candidate` | Obstacle-free leaf: try `kani_autoharness` (`references/autoharness.md`) |
| `harness-with-callee-contracts` | Write a harness; cover callees with verified contracts |

## Limits

- Dynamic dispatch, closures, macro-generated code, FFI, `unsafe`, `#[cfg]` differences, and generic
  instances are *not* exact in the map. They appear as `uncertainty` and as unresolved edges.
  Do not fill the gaps by assuming the callee is fine.
- The text-scan call graph resolves method calls by name only and ignores `#[cfg]`.
  `cargo-callgraph` is rust-analyzer based and uses its own default feature set. Neither is the
  compiler's call graph.
- A map is a snapshot. A function whose callee changed is not re-checked by `update`.
- The design and the schema are in `docs/verifiability-map-design.md` and
  `schemas/verifiability-map.schema.json`.
