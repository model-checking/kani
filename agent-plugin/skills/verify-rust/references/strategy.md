# Choosing a proof strategy

The default is **contract-first**, because it scales and it gives unbounded proofs:

1. a **contract** for each function (`requires`, `ensures`, `modifies`);
2. a **loop invariant and a `decreases` clause** for each loop, instead of an unwinding bound;
3. **AutoHarness** verifies every contract with no hand-written harness;
4. callers use the **verified contract** of their callees (`stub_verified`), not their bodies.

A hand-written harness and `#[kani::unwind(N)]` are the fallback: for a property over several
calls, a scenario, an environment that needs stubs, or a loop whose invariant cannot be found. A
bounded proof is a weaker result: say so in the report.

## The plan: the Crate Verifiability Map

For more than a few functions, build the map (`references/verifiability-map.md`) and plan from it.
It gives the call graph, the recursive components, and, per function, what is missing: a contract,
a loop invariant, a decreases clause, a verified callee. Its plan is bottom-up: each stage has the
steps and the AutoHarness command that verifies the stage's contracts.

For each function, in the plan's order:

- **Leaf with no loops:** write the contract, verify it with AutoHarness
  (`write-contract-then-autoharness`).
- **Has loops:** invariant, `loop_modifies` if it writes through references, and
  `#[kani::loop_decreases(...)]` (`loop-contract`, `add-loop-decreases`). The invariant must hold on
  entry and after every iteration, and, together with the exit condition, imply what comes after the
  loop. The decreases expression must strictly decrease and stay bounded below. Without a decreases
  clause the proof is partial (termination is not shown); without an invariant it holds only within
  the unwind bound.
- **Recursive (alone or with others):** a contract for every member of the component and
  `#[kani::recursion]` on the recursive functions (`contract-and-recursion`). The contract is what
  callers inside the cycle use.
- **Calls other workspace functions:** verify the callees first (earlier stages), then call them
  through `#[kani::stub_verified(callee)]` in a harness, or let AutoHarness verify the caller's own
  contract (it inlines callees unless they are stubbed in a manual harness). A callee whose contract
  is not verified is an assumption: the map reports the caller as `conditional`.
- **Calls trait objects, closures, generics, or foreign code** (the map lists these as unresolved
  callees): give the trait method a contract on each implementation you rely on and verify each;
  instantiate generics with the concrete types you care about; stub only the narrow function that
  cannot be modeled (environment, process, file), with `#[kani::stub]`, and report the stub.
- **Unbounded inputs** (`&[T]`, `&str`, `Vec`, `String`): bound them in a `requires`, or with
  `bounded_any` and `--bounded-arguments`; say the bound in the report.
- **`unsafe`:** state the safety conditions as `requires`, check `-Z valid-value-checks` and
  `-Z uninit-checks`, and confirm UB defects with Miri.

## One function

For a single function with no loops, write its contract and run AutoHarness on it
(`--include-pattern '^crate::path$'`). For a property that is not a contract of one function, write
one harness (`references/kani.md`).

## Sweeping a crate

AutoHarness checks every eligible function for panics and UB on all inputs, and verifies each
declared contract (`references/autoharness.md`). Run it first on a crate you do not know: its
failures are either defects or missing preconditions, and each missing precondition is a contract
to write. Prefer `--bounded-arguments` for functions taking slices and strings, and say the bounds.

## Stubs and expensive callees

If a harness is slow, look in the call graph for callees with loops, large allocations, or heavy
arithmetic (hashing, parsing). Give them a contract and verify it, then use `stub_verified`. A
plain `#[kani::stub]` with a hand-written model is an unverified assumption: use it only with the
user's approval, for what cannot be modeled, and list it in the report.

## When the property cannot be proved as stated

If the proof needs a precondition or assumption the user did not approve (a narrower input range, a
platform assumption such as a 64-bit `usize`, a precondition on input contents), stop and ask.
Present the exact `requires`, why it is needed (the failing check and a concrete witness), and what
it would exclude. A refused precondition often points at a real defect: confirm it with
`scripts/check-witness.py`.
