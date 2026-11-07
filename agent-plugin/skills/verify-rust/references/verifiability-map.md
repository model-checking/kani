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
- `contract_status`, `signals.loop_invariants`, `signals.loop_decreases`: how far the function is
  along the contract-first path. `verification.caveats` list what the result relies on.
- `complexity`: reserved for the five metrics of the proposed `cargo kani complexity` command, and
  `not_computed` until it exists. `signals` are different: text-based counts (loop and branch
  keywords, `unsafe`, unbounded input types) that guide the heuristic.

## Contract-first

The map's default is contract-first (`references/strategy.md`, `references/contracts.md`): every
function gets a contract, every loop an invariant and a decreases clause, and AutoHarness verifies
the contracts bottom-up. For each function the map records `contracts`, `contract_status` (`none`,
`declared`, `verified`, `failed`, ...), and the loop signals `loop_keywords`, `loop_invariants`,
`loop_decreases`, `loop_modifies`. `kani_plan_verification` gives each stage its steps, the loops
that still need contracts (`loop_contract_work`), and an `autoharness` block with the include
patterns, the CLI command, and the MCP call that verify the stage. Its `contract_progress` counts
contracts verified and failed, loops with invariants and with decreases clauses, and the functions
whose proofs are partial (`partial_termination`). A loop without an invariant is verified only
within the unwind bound, and the function's `verification.caveats` say so. AutoHarness results
(from `kani_autoharness` or `cargo kani autoharness -Z export-json`) are matched to their functions
and count as evidence.

## MCP tools

`kani_map_crate` (build; returns a summary and the map's path), `kani_plan_verification` (stages,
with `goals` and `max_items`), `kani_update_verification_map` (merge new exports), and
`kani_explain_function`. The map stays on disk; the tools return compact answers. They run the
plugin's scripts: set `KANI_PLUGIN_DIR` to the plugin directory if the server is not started from it.

## Strategies

| `primary` | Do |
|-----------|----|
| `triage-failure` | Replay the counterexample (`kani_get_counterexample`) and decide: bug, or missing precondition |
| `reduce-or-split` | The run was inconclusive: shrink bounds, stub expensive callees, split the harness (`references/kani.md`, performance) |
| `avoid-unsupported-construct` | Stub or restructure what Kani does not support |
| `re-run` | The evidence is stale: re-run the harness |
| `contract-and-recursion` | Write `requires`/`ensures` for every member of the recursive component, mark the recursive functions `#[kani::recursion]`, verify with AutoHarness (`references/contracts.md`) |
| `loop-contract` | Write `#[kani::loop_invariant]`, `loop_modifies` if needed, and `#[kani::loop_decreases]`; `unwind` only as a fallback |
| `add-loop-decreases` | The loops have invariants but no decreases clause: the proof is partial |
| `instantiate-concrete-types` | Verify contracts for concrete instantiations of the generic |
| `stub-or-contract-callees` | Callees are trait-level, dynamic, or unknown: give them contracts and use `stub_verified`; a plain stub is an unverified assumption |
| `bounded-inputs` | Bound slices, strings, `Vec`s in a `requires`, or with `bounded_any` / `--bounded-arguments` |
| `ub-focused-harness` | State safety conditions as `requires`; also use `-Z valid-value-checks` and `-Z uninit-checks` |
| `verify-contract-with-autoharness` | The contract exists: verify it with `kani_autoharness` (`references/autoharness.md`) |
| `write-contract-then-autoharness` | No contract yet: write one (show it to the user), then verify it with AutoHarness |
| `run-existing-harness` | A hand-written harness exists but has no result yet |

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
