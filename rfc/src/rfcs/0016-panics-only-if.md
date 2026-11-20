- **Feature Name:** Panic conditions in function contracts (`panics-only-if`)
- **Feature Request Issue:** [#3567](https://github.com/model-checking/kani/issues/3567)
- **RFC PR:** [#3893](https://github.com/model-checking/kani/pull/3893)
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** Not yet. [#4230](https://github.com/model-checking/kani/pull/4230) prototypes a
  `panics_if` clause with different semantics (see [Rationale and alternatives](#rationale-and-alternatives)).

-------------------

## Summary

A new function contract clause, `#[kani::panics_only_if(cond)]`, states the only conditions under which a
function is allowed to panic. Kani proves three things:
- the function is free of undefined behavior on every path, including paths that panic;
- it panics only if `cond` held on entry;
- its postconditions hold whenever it returns.

## User Impact

Today a function contract has no way to say that a function is *meant* to panic in some cases. A
`proof_for_contract` harness reports every reachable panic as a verification failure, so users can only verify
functions such as `Option::unwrap`, `<[T]>::index` or `Vec::insert` in one of these ways:

- **Strengthen `requires` until no panic is reachable.**
  - This hides the panicking inputs from verification, including any undefined behavior on the way to the panic.
  - It misrepresents the contract: callers may pass those inputs, they just get a panic.
  - With `stub_verified`, it also forces every caller to prove that it never passes them.
- **Add `#[kani::should_panic]` to the harness.**
  - This accepts *any* panic.
  - It also accepts failures that are not panics (see
    [Prerequisite](#prerequisite-telling-panics-apart-from-kani-checks)).
  - It requires at least one panic to happen.
- **Run with `-Z unstable-options --prove-safety-only`** ([#4239](https://github.com/model-checking/kani/pull/4239)).
  - This assumes that no panic occurs anywhere. It checks the absence of undefined behavior, but it is a global,
    per-run switch.
  - It cannot record or check when a given function may panic.
  - A harness verified this way says nothing about panics to the callers of a function.

Rust APIs document their panics in the `# Panics` section of their documentation, and whether a function can
panic matters to its callers. With `panics_only_if`, users can:

1. Prove the absence of undefined behavior for functions that are expected to panic for some inputs, without
   excluding those inputs.
2. State, and have Kani check, the conditions under which a function may panic. A panic outside those conditions
   is reported, so unexpected panics are still found.
3. Prove total correctness: together with `ensures`, describe exactly when a function panics and what it returns
   otherwise.
4. Use these specifications modularly. When `stub_verified` replaces a function with its contract, the
   replacement may panic exactly when the contract allows it, so callers have to account for that panic.

Downsides:
- This is another contract clause to learn.
- Its condition is a necessary rather than a sufficient condition for a panic, which needs to be documented well
  (see [Naming](#naming)).
- It relies on a change to how Kani classifies its checks, which changes some property names in Kani's output.

## User Experience

The clause is part of function contracts and is available with `-Z function-contracts`.

```rust
#[kani::panics_only_if(cond)]
```

`cond` is a Boolean expression with the same restrictions as a `requires` condition:
- it may refer to the function's arguments;
- it may call side-effect-free functions;
- it may not refer to the result;
- it is evaluated on entry to the function, after the preconditions have been assumed.

A function may have at most one `panics_only_if` clause. A function without the clause behaves as it does today,
which is equivalent to `#[kani::panics_only_if(false)]`.

### What Kani checks

Take a harness `#[kani::proof_for_contract(f)]`, where `f` has the contract `requires(P)`, `panics_only_if(C)`
and `ensures(Q)`. For all inputs that satisfy `P`, Kani checks that:

1. No undefined behavior occurs on any path through `f`, including on paths that end in a panic, up to the panic.
2. If `f` panics, `C` held on entry to `f`.
3. If `f` returns, `Q` holds. This applies whether or not `C` held: `C` permits a panic, it does not excuse a
   wrong result.
4. `f` modifies only what its `modifies` clauses allow.

`C` is a necessary condition for a panic, not a sufficient one: Kani does not check that `f` panics when `C`
holds. To also require that, add a postcondition saying that `f` returns only if `C` did not hold (see the
examples).

A panic that `C` does not permit is reported as a failed check at the panic site. The check carries the panic's
own message and the `panics_only_if` condition, for example:

```
Check 3: core::option::unwrap_failed.panic.1
	 - Status: FAILURE
	 - Description: "called `Option::unwrap()` on a `None` value (not permitted by `panics_only_if(self.is_none())`)"
```

### Examples

`Option::unwrap` may only panic when the option is `None`, and returns the contained value otherwise (shown for
`T: Copy + PartialEq` to keep the postcondition short):

```rust
#[kani::panics_only_if(self.is_none())]
#[kani::ensures(|result| old(self) == Some(*result))]
pub fn unwrap(self) -> T { /* ... */ }
```

The postcondition cannot hold when `self` is `None`. So this contract also implies that `unwrap` panics whenever
`self` is `None`; the alternative, never returning, is ruled out by the harness's unwinding assertions.

Where the postcondition does not imply the panic, add one that does. `Vec::insert` panics if `index > len`:

```rust
#[kani::panics_only_if(index > self.len())]
#[kani::ensures(|_| index <= old(self.len()))] // returning implies the panic condition was false
#[kani::ensures(|_| self.len() == old(self.len()) + 1)]
pub fn insert(&mut self, index: usize, element: T) { /* ... */ }
```

A function whose panics are not the property of interest can permit all of them. Kani then still proves the
absence of undefined behavior, and the postcondition on every return, but not panic freedom:

```rust
#[kani::panics_only_if(true)]
#[kani::ensures(|result| result.is_sorted())]
fn sort_with<T: Ord>(v: Vec<T>, is_less: impl FnMut(&T, &T) -> bool) -> Vec<T> { /* ... */ }
```

### Contract replacement

`stub_verified(f)` replaces `f` with its contract, and Kani does the same for recursive calls. The replacement
does three things:

1. It checks `P`, as today.
2. It evaluates `C`. If `C` holds, the replacement either panics or continues, nondeterministically.
3. Otherwise, or if it continues, it havocs the `modifies` targets and assumes `Q`, as today.

The panic in step 2 is a panic of the caller at the call site, so the caller's own contract governs it.
Verification of the caller fails at that call if the caller has no `panics_only_if` clause, or has one that does
not permit this panic. Callers therefore account for the panics their callees are allowed to have. That is what
makes the clause sound to use modularly.

### Interaction with other features

- **`#[kani::should_panic]`** keeps its meaning: verification succeeds only if at least one panic occurs and every
  failed check is a panic. A panic that `panics_only_if` does not permit is a failed check that is not an
  expected panic, so the harness fails.
- **`--prove-safety-only`** assumes that no panic occurs:
  - `panics_only_if` checks nothing extra under it;
  - the replacement in step 2 never panics;
  - postconditions are still checked (this requires the prerequisite below).
- **`autoharness`** generates `proof_for_contract` harnesses for functions with contracts, and those harnesses
  check the clause like any other.
- **Multiple harnesses** for the same function all check the clause, as they do for `requires` and `ensures`.

### Errors

- A second `panics_only_if` clause on the same function is a compile error.
- `panics_only_if` on a function with no `proof_for_contract` harness is reported like any other contract
  without a harness.
- If `cond` refers to the result or is not a Boolean expression, that is a compile error, as for `requires`.

### What counts as a panic

A panic is any path that reaches Rust's panic machinery as Kani models it. That covers:
- `panic!`, `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!` and their variants, `unreachable!`, `todo!` and
  `unimplemented!`;
- library functions that panic, such as `Option::unwrap` and `Result::expect`;
- the panics rustc inserts for bounds checks, arithmetic overflow, division or remainder by zero, and invalid
  shifts.

The following are not panics:
- undefined behavior (safety checks);
- Kani's own checks: `kani::assert`, `kani::check`, contract pre- and postconditions, and `kani::cover`;
- unsupported constructs;
- aborts such as `std::process::abort`.

As today, Kani compiles code with overflow checks and debug assertions enabled, so overflow and `debug_assert!`
failures are panics. See [Open questions](#open-questions) for builds where they are disabled.

### Output changes

Rust panics and Kani's own checks no longer share the `assertion` property class (see the
[prerequisite](#prerequisite-telling-panics-apart-from-kani-checks)). This changes some property names in Kani's
output, in `--export-json` and in SARIF. It does not change the description of any check, and it changes only
verification results that are wrong today:
- `should_panic` harnesses that pass only because a `kani::assert` or a postcondition failed now fail;
- `--prove-safety-only` now checks postconditions and `kani::assert`.

## Software Design

### Prerequisite: telling panics apart from Kani checks

Kani currently emits Rust panics and its own checks with the same property class, `assertion`:
- `codegen_panic` (the panic hook and `PanicStub`) uses it for panics;
- MIR `Assert` terminators use it for bounds, overflow and division checks;
- the `kani::assert` hook uses it for `kani::assert`. That hook also serves Kani's `assert!`, `assert_eq!` and
  `assert_ne!` overrides in `library/std`, and contract postconditions expand to `kani::assert`.

So neither the compiler nor the driver can tell a panic from a failed postcondition. Two existing features are
wrong because of this:
- `should_panic` accepts a failed postcondition or `kani::assert` as an expected panic.
- `--prove-safety-only` assumes away `assertion` checks, so it also assumes away postconditions and `kani::assert`.
  For example, a `proof_for_contract` harness verifies under `--prove-safety-only` even when the function
  violates its `ensures` clause.

The first step, which is useful on its own ([#5212](https://github.com/model-checking/kani/issues/5212)), is to give panics their own property class, `panic`:
- `codegen_panic` and the panicking MIR assertions use `panic`.
- The `assert!`, `assert_eq!` and `assert_ne!` overrides call a new Kani-internal function, hooked to emit a
  `panic` check, instead of `kani::assert`.
- `kani::assert` and contract conditions keep `assertion`.
- The driver's `should_panic` classification and `--prove-safety-only` use `panic` instead of `assertion`.

### Encoding

The check needs a single verification run per harness.

1. **Macro expansion (`kani_macros`).** `panics_only_if` is parsed like `requires`.
   - In the check closure, the condition is evaluated after the preconditions are assumed. Its value is stored in
     a Kani-internal global, `PANIC_PERMITTED: bool`, and the previous value is restored when the function
     returns.
   - The global is `false` everywhere else, so panics in the harness itself, before or after the call, are
     checked as today.
   - In the replace closure, the condition is evaluated after the preconditions are checked. If it holds, the
     closure panics nondeterministically through the panic machinery, so the panic is a `panic` check of the
     caller.
2. **Codegen (`kani-compiler`).** This step applies to a codegen unit whose harness is a `proof_for_contract` for
   a function with a `panics_only_if` clause. Other codegen units are unchanged.
   - Every `panic` check becomes `assert(PANIC_PERMITTED)` followed by `assume(false)`, instead of `assert(false)`.
     The assertion keeps the panic's message and location and mentions the condition.
   - The path ends at the panic either way, as today.
   - Safety checks, postconditions and every other check are unchanged and are evaluated on all paths. So
     undefined behavior on the way to a permitted panic is reported.
3. **Driver (`kani-driver`).** No change beyond the `should_panic` classification above. A failed
   `assert(PANIC_PERMITTED)` is a failed check like any other.

Only the function under verification sets `PANIC_PERMITTED`. Two consequences:
- Panics in its callees count as its own panics, since they propagate through it.
- A callee replaced by its contract panics only when its own clause permits it, and the caller's flag then
  checks that panic.

### Corner cases

- **Recursion.** Recursive calls are replaced by the contract. Step 2 of the replacement therefore applies to them,
  and their permitted panics are checked against the outer call's condition.
- **Closures and generic functions** passed to the function under verification are part of its body. A panic in
  a user-supplied comparator is a panic of the function, and is permitted only if the condition allows it. This
  is the intended reading for APIs such as sorting, where `panics_only_if(true)`, or a narrower condition, has to
  account for panicking user code.
- **`kani::any_where` and `kani::assume` in the harness** constrain inputs as today, and the condition is
  evaluated on the constrained inputs.
- **Loop contracts and quantifiers** are unaffected, because the global flag is ordinary state.

## Rationale and alternatives

### A precondition-style `panics_if` (#4230)

The prototype in #4230 assumes `!cond` when checking the function and asserts `!cond` when replacing it. That
makes the clause a precondition by another name:
- The inputs that satisfy `cond` are not verified at all. Undefined behavior on those paths, including before
  the panic, is hidden.
- Callers must prove they never pass such inputs, which is not what a panicking API promises.

This RFC keeps all inputs in scope. It only changes whether reaching a panic is a failure.

### A "must panic" clause (`panics_if`)

#3567 suggested a clause that requires the function to panic whenever `cond` holds. It gives a stronger
guarantee, but it cannot always be stated. Whether a function panics can depend on state, or on user code that
the condition cannot refer to. An example is a comparator that panics for some pairs, or that is not a total
order. The weaker clause covers those cases, and the stronger guarantee is still available by adding a
postcondition, as in the `insert` example. A dedicated `panics_if` can be added later as sugar for that
combination.

### Running verification twice

The first version of this RFC proposed two runs:
- one assuming that the condition does not hold, which checks that no panic occurs;
- one without assumptions that accepts only panics, like `should_panic`.

The single-run encoding above checks the same properties in one run. It also does not rely on `should_panic`'s
classification, which today accepts failures that are not panics.

### Reusing `--prove-safety-only`

`--prove-safety-only` remains the right tool for a quick, global check for undefined behavior. But it cannot
express which panics a function is allowed to have, it does not report unexpected panics, and it does not let the
contract of a function inform its callers.

### Naming

The condition is necessary for a panic, not sufficient. Earlier names were read the other way:
- `may_panic_if(cond)` reads as "if `cond`, the function may panic". It does not say that the function may not
  panic otherwise.
- `should_panic`'s "should" already means "must" in Kani.

`panics_only_if(cond)` states the implication in the direction Kani checks: the function panics only if `cond`
holds. Alternatives considered:
- `no_panic_unless(cond)`, which says the same thing;
- its dual, `no_panic_if(!cond)`, which states the panic-free inputs instead of the panicking ones.

### Multiple clauses

Several `panics_only_if` clauses would have to be OR-ed, which is the opposite of how several `requires` or
`ensures` clauses combine. To avoid that confusion, the first version allows one clause, and users can write `||`.

## Open questions

- **Builds without overflow checks or debug assertions.** In such builds, overflow wraps and `debug_assert!` does
  not run, so code after them executes, while Kani ends the path at the panic. This is also true of every Kani
  harness today. `--prove-safety-only` turns off debug assertions for this reason
  ([#4262](https://github.com/model-checking/kani/pull/4262)). Should `panics_only_if` offer, or default to,
  checking safety under both configurations?
- **Granularity.** Should users be able to permit only some kinds of panics, such as arithmetic overflow, or to
  match panic messages?
- **Mutable arguments.** The condition is evaluated on entry. Is there a need to refer to the state at the time of
  the panic?

## Out of scope / Future Improvements

- **Panic safety.** Kani compiles code with `-C panic=abort` and does not explore unwinding. Drop glue and
  `catch_unwind` handlers that run during unwinding are therefore not checked.
  - A permitted panic says nothing about undefined behavior or broken invariants while unwinding. That is the
    concern for sorting with a panicking comparator
    ([#3567](https://github.com/model-checking/kani/issues/3567#issuecomment-2569213652),
    [verify-rust-std Challenge 8](https://github.com/model-checking/verify-rust-std/issues/56)).
  - Supporting it needs unwinding support in Kani and a postcondition on the panicking exit, comparable to SPARK's
    `Exceptional_Cases` aspect or JML's `signals` clauses.
- **A `panics_if` shorthand** for `panics_only_if` plus the postcondition that makes the panic mandatory.
- **Checking `# Panics` documentation** against the contract.
