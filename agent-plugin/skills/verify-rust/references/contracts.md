# Function contracts, loop contracts, and termination

Contracts let Kani verify a function once, against a specification, and let every caller use that
specification instead of the body. They are unstable: `-Z function-contracts` and
`-Z loop-contracts` (AutoHarness implies both). The MCP tools and scripts add the flags they detect.

## Function contracts

```rust
#[kani::requires(divisor != 0)]                        // what callers must guarantee
#[kani::ensures(|result: &u32| *result <= dividend)]   // what the function promises
#[kani::modifies(&mut *out)]                           // what it may write (references, pointers)
fn div(dividend: u32, divisor: u32) -> u32 { dividend / divisor }
```

- **Verify a contract with AutoHarness**: it generates the `proof_for_contract` harness. By hand:
  `#[kani::proof_for_contract(div)] fn check_div() { div(kani::any(), kani::any()); }`. The
  harness needs no `kani::assume` for the `requires`: Kani assumes it.
- **Use a verified contract in a caller's harness**: `#[kani::stub_verified(div)]` replaces `div`
  by its contract. It is sound only if `div`'s contract is verified (the map reports a caller as
  `conditional` until it is).
- **`requires` is an assumption.** Every clause narrows what is proved. Keep each as weak as the code
  needs, show them to the user, and check them: `scripts/check-specs.py` asks Kani whether the
  preconditions are satisfiable together (a contradictory `requires` proves nothing) and whether a
  clause constrains nothing. `check-proof-diff.py` flags a strengthened `requires` or a removed
  `ensures`.
- **`ensures`** takes a closure over the result (`|result: &T| ...`) and can use the function's
  parameters. State the property, not the implementation: a postcondition that restates the body
  proves little.
- **`modifies`** is required for functions that write through arguments. Leave out what the
  function must not touch.
- **Recursive functions** need contracts on every member of the recursive group and the
  `#[kani::recursion]` attribute on the recursive functions. Without it, the contract check
  re-enters itself.
- **Methods and trait implementations**: a contract on a method is checked for `self` of any value
  the type's `Arbitrary` implementation (or `#[derive(kani::Arbitrary)]`) generates. For a trait,
  give a contract to each implementation you rely on.
- **Generics**: a contract is verified for the instances you check, not for all `T`.

## Loop contracts and termination

Loop contracts abstract a loop, so the proof does not unroll it and does not depend on an
unwinding bound or on a bound on the input:

```rust
#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn count_down(n: u32) -> u32 {
    let mut x = n;
    #[kani::loop_invariant(x <= n)]        // holds before the loop and after every iteration
    #[kani::loop_decreases(x)]             // strictly decreases each iteration, bounded below by 0
    while x > 0 {
        x -= 1;
    }
    x
}
```

- **`#[kani::loop_invariant(cond)]`**: must hold on entry and be preserved by the body; together
  with the negated loop guard it must imply what the code after the loop needs. Several attributes
  combine. Kani checks it is inductive.
- **`#[kani::loop_modifies(var, ...)]`** lists what the loop writes through references, when the
  invariant needs it.
- **`#[kani::loop_decreases(expr, ...)]`** is the termination argument: `expr` must strictly
  decrease on every iteration and stay bounded below (several expressions are ordered
  lexicographically). **Without it a proof that uses the invariant is partial**: it holds assuming
  the loop terminates, and a non-terminating loop makes later assertions vacuously true. The map
  flags this as `partial-termination`.
- Put the attributes on a `while`, `loop`, or `for` loop. In a `for` loop over an iterator,
  `kani::index` is the position of the current iteration (`#[kani::loop_invariant(sum <= kani::index as u32 * 20)]`).
- **Historic values** in a loop contract: `on_entry(expr)` is the value before the loop, and
  `prev(expr)` the value in the previous iteration (Kani then asserts the loop runs at least once).
  Quantifiers (`kani::forall!`, `kani::exists!`, with `-Z quantifiers`) state facts about arrays.
- Loop contracts work inside functions that have function contracts: the loop invariant proves the
  postcondition.
- The crate needs `#![feature(stmt_expr_attributes, proc_macro_hygiene)]` (nightly features, which
  Kani's toolchain allows) and `-Z loop-contracts`. Guard it: `#![cfg_attr(kani, feature(...))]`, and
  put the loop attributes behind `#[cfg_attr(kani, kani::loop_invariant(...))]` so ordinary builds
  are unchanged.
- If you cannot find an invariant, fall back to `#[kani::unwind(N)]`, with `N` greater than the
  maximum number of iterations: the result then holds only for inputs within that bound, and the
  report must say so.

## What each step proves, and what it does not

| You write | The proof covers | Not covered |
|-----------|------------------|-------------|
| contract, verified by AutoHarness | every input `requires` allows, including unbounded loops with invariants | inputs `requires` excludes; callees stubbed or assumed |
| loop invariant, no decreases | partial correctness | termination |
| invariant and decreases | total correctness of the loop | what the invariant does not mention |
| `stub_verified(g)` in a harness | the caller, assuming `g`'s verified contract | `g`'s body (verified separately) |
| `#[kani::unwind(N)]` | inputs within the bound | larger inputs |
