# Choosing a proof strategy

Pick the smallest strategy that proves the agreed property. More structure costs more turns and
more context; it pays off only when one harness cannot handle the code.

## One harness (small targets)

For a single function, or a few functions with a short call chain, write one harness that calls
the entry point with nondeterministic inputs and asserts the property. Kani analyzes the whole
call tree. This is usually the cheapest path, and the right first attempt.

## Contracts, bottom-up (larger targets)

When one harness is too slow, needs unwinding bounds that are too large, or the same callee is
reached from many places, verify functions one at a time against contracts
(`references/contracts.md`):

1. **Map the call graph** of the code under verification with rust-analyzer's call hierarchy
   (`references/rust-analyzer.md`): for each function, its callees inside the crate.
2. **Start at the leaves**: functions that call nothing else in scope. Give each a contract
   (`#[kani::requires]`, `#[kani::ensures]`, `#[kani::modifies]`) and prove it with a
   `#[kani::proof_for_contract]` harness.
3. **Move up the graph**: prove each caller with `#[kani::stub_verified(callee)]` for every
   callee whose contract is proved. Kani then uses the callee's contract instead of its body.
4. **Keep every edge consistent**: a callee's `requires` must hold at each call site, and the
   caller may rely only on the callee's `ensures`. If a caller needs more, strengthen the callee's
   `ensures` (and re-prove it), rather than weakening the caller's property.
5. **Check every contract** with `scripts/check-specs.py`: a precondition no input satisfies
   proves nothing, and `check-specs.py --compare-base` shows whether a changed precondition
   now excludes inputs it used to admit.

Recursive functions need contracts; cycles in the call graph show where.

## Stubs and expensive callees

If a harness is slow, look in the call graph for callees with loops, large allocations, or heavy
arithmetic (hashing, crypto, parsing). Prefer proving them with a contract and using
`stub_verified`. Use `#[kani::stub]` with a hand-written model only with the user's approval, and
list it in the report: an unverified stub is an assumption about the real code.

## Sweeping a crate

To find panics and UB across many functions before writing any harness, run AutoHarness
(`references/autoharness.md`). Turn each real finding into a targeted harness or contract.

## When the property cannot be proved as stated

If the proof needs an assumption the user did not approve (a narrower input range, a platform
assumption such as a 64-bit `usize`, a precondition on input contents), stop and ask. Present the
exact assumption, why it is needed (the failing check and a concrete witness), and what it would
exclude. A refused assumption often points at a real defect: confirm it with
`scripts/check-witness.py`.
