- **Feature Name:** Panic conditions in function contracts (`panics-only-if`)
- **Feature Request Issue:** [#3567](https://github.com/model-checking/kani/issues/3567)
- **RFC PR:** [#3893](https://github.com/model-checking/kani/pull/3893)
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** [#4230](https://github.com/model-checking/kani/pull/4230)

-------------------

## Summary

Two new function contract clauses describe when a function panics:
- `#[kani::panics_only_if(cond)]` states the only condition under which the function may panic;
- `#[kani::panics_if(cond)]` states a condition under which it must panic.

For a function with these clauses, Kani proves that:
- the function is free of undefined behavior on every path, including paths that panic;
- it panics only when the clauses permit it, and does not return when `panics_if` requires a panic;
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

Rust APIs document their panics in the `# Panics` section of their documentation, usually as "Panics if ...",
and whether a function can panic matters to its callers. With the new clauses, users can:

1. Prove the absence of undefined behavior for functions that are expected to panic for some inputs, without
   excluding those inputs.
2. State, and have Kani check, exactly when a function panics: `panics_if(cond)` alone means "panics if and
   only if `cond`", the direct translation of a `# Panics` section. A panic outside the condition, or a return
   when the condition held, is reported.
3. Describe functions whose panics cannot be stated exactly, such as functions that call user code, with
   `panics_only_if`: unexpected panics are still found.
4. Use these specifications modularly. When `stub_verified` replaces a function with its contract, the
   replacement panics as the contract says, so callers have to account for those panics.

Downsides:
- These are two more contract clauses to learn, and the difference between a necessary condition
  (`panics_only_if`) and a sufficient one (`panics_if`) needs to be documented well (see [Naming](#naming)).
- They rely on a change to how Kani classifies its checks, which changes some property names in Kani's output.

### Evidence from verify-rust-std

We surveyed [verify-rust-std](https://github.com/model-checking/verify-rust-std) (main at `a43ef3d40d9`, 1890
harnesses) with the Kani version it pins.

**`should_panic` rarely tests what it seems to.** The 28 `#[kani::should_panic]` attributes expand to 116
harnesses. Only 27 pass because the function under verification panics. The other 89 pass for unrelated reasons:
- 79 because the harness itself panics, for example in `kani::any::<Duration>()` (where `Duration::new` panics) or
  in `unwrap()` on an expected `Err`;
- 2 because the contract panics: `offset_from::<()>`'s `requires` divides by zero
  (`library/core/src/ptr/const_ptr.rs:649`);
- 8 because a check that is not a panic fails: the `should_fail_*` harnesses in
  `library/core/src/intrinsics/mod.rs` rely on a failed `stub_verified` precondition.

Fourteen of these are `proof_for_contract` harnesses whose target has `ensures` clauses, so a broken postcondition
goes unnoticed ([#5212](https://github.com/model-checking/kani/issues/5212)). We confirmed this on five std
functions. For example, we changed `Duration::as_secs` (`library/core/src/time.rs:615`) to return
`secs.wrapping_add(1)`. `duration_as_secs` then fails its postcondition, but `duration_as_secs_panics` still passes:
it lists the failed postcondition among the failures it accepts as expected panics.

**Restrictions hide inputs that are safe to verify.** Of the 191 functions with a `requires`, 8 are safe. Three
of those exclude exactly the inputs that their `# Panics` documentation names, for example `copy_from_slice`'s
`requires(self.len() == src.len())` (`library/core/src/slice/mod.rs:4385`). Eleven `kani::assume` calls do the same
in harnesses, and one sits in library code (`library/core/src/num/imp/dec2flt/decimal_seq.rs:183`). We removed
the restriction in 16 harnesses covering 13 functions and ran them with `--prove-safety-only`. None reported
undefined behavior on the panicking inputs, and each run took under 6 seconds. Without the flag, every failure
was a panic. So for these functions the restriction excluded panics only, and it also kept their absence of
undefined behavior from being checked on those inputs.

**The conditions are easy to state.** For 12 of the 13 panicking functions we found, the `# Panics` condition is
a simple expression over the arguments, such as `index >= self.len()`, `!align.is_power_of_two()` or
`self.get() == MIN`, so `panics_if` alone describes them exactly. The exception is `NonNull::map_addr`, which
panics if its closure does: its condition can only be stated as `panics_only_if`. For `NonZero::abs` and `Neg`,
the panic is an overflow check, so `panics_if(self.get() == MIN)` holds when overflow checks are enabled, as they
are when Kani verifies code (see [Open questions](#open-questions)).

## User Experience

The clauses are part of function contracts and are available with `-Z function-contracts`.

```rust
#[kani::panics_only_if(cond)]   // the function may panic only if `cond` holds
#[kani::panics_if(cond)]        // the function must panic if `cond` holds
```

Each `cond` is a Boolean expression with the same restrictions as a `requires` condition:
- it may refer to the function's arguments;
- it may call side-effect-free functions;
- it may not refer to the result;
- it is evaluated on entry to the function, after the preconditions have been assumed.

A function may have at most one clause of each kind. A function without either clause behaves as it does today:
it may not panic.

### What Kani checks

Take a harness `#[kani::proof_for_contract(f)]`, where `f` has the contract `requires(P)`, `panics_only_if(C)`,
`panics_if(Q)` and `ensures(R)`. Without a `panics_only_if` clause, `C` is `Q`; without a `panics_if` clause, `Q`
is `false`. For all inputs that satisfy `P`, Kani checks that:

1. No undefined behavior occurs on any path through `f`, including on paths that end in a panic, up to the panic.
2. If `f` panics, `C` held on entry to `f`.
3. If `f` returns, `Q` did not hold on entry: if it held, `f` must have panicked.
4. If `f` returns, `R` holds. This applies whether or not `C` held: `C` permits a panic, it does not excuse a
   wrong result.
5. `f` modifies only what its `modifies` clauses allow.

So `panics_if(Q)` alone states that `f` panics if and only if `Q` holds, and `panics_only_if(C)` alone states
that `f` may panic only if `C` holds. With both clauses, the inputs where `C` holds but `Q` does not are those
where `f` may or may not panic. `Q` must imply `C`: for an input where `Q` holds but `C` does not, `f` can
neither panic (check 2) nor return (check 3), so any such input is reported.

A panic that the clauses do not permit is reported as a failed check at the panic site, with the panic's own
message:

```
Check 3: core::option::unwrap_failed.panic.1
	 - Status: FAILURE
	 - Description: "called `Option::unwrap()` on a `None` value (not permitted by the contract)"
```

A return when `panics_if` required a panic is reported as a failed check with the description
``returned although `panics_if(index > self.len())` held``.

### Examples

`Vec::insert` panics if and only if `index > len`:

```rust
#[kani::panics_if(index > self.len())]
#[kani::ensures(|_| self.len() == old(self.len()) + 1)]
pub fn insert(&mut self, index: usize, element: T) { /* ... */ }
```

`Option::unwrap` panics if and only if the option is `None` (shown for `T: Copy + PartialEq` to keep the
postcondition short):

```rust
#[kani::panics_if(self.is_none())]
#[kani::ensures(|result| old(self) == Some(*result))]
pub fn unwrap(self) -> T { /* ... */ }
```

A function whose panics cannot be stated exactly can bound them. A sort that calls a user comparator may panic
whenever the comparator does, so it permits every panic, and Kani still proves the absence of undefined behavior
and the postcondition on every return:

```rust
#[kani::panics_only_if(true)]
#[kani::ensures(|result| result.is_sorted())]
fn sort_with<T: Ord>(v: Vec<T>, is_less: impl FnMut(&T, &T) -> bool) -> Vec<T> { /* ... */ }
```

Both clauses together describe a function that must panic for some inputs and may panic for others. This one
must panic if the index is out of bounds, and may also panic if the user's closure does:

```rust
#[kani::panics_only_if(true)]
#[kani::panics_if(index >= self.len())]
fn update(&mut self, index: usize, f: impl FnOnce(&mut T)) { /* ... */ }
```

### Contract replacement

`stub_verified(f)` replaces `f` with its contract, and Kani does the same for recursive calls. The replacement
does three things:

1. It checks `P`, as today.
2. It evaluates `Q` and `C`. If `Q` holds, it panics. Otherwise, if `C` holds, it either panics or continues,
   nondeterministically.
3. If it continues, it havocs the `modifies` targets and assumes `R`, as today.

The panic in step 2 is a panic of the caller at the call site, so the caller's own contract governs it.
Verification of the caller fails at that call if the caller has no clause that permits this panic. Callers
therefore account for the panics their callees are allowed or required to have. That is what makes the clauses
sound to use modularly. Conversely, a caller may rely on a required panic: after a call that returns, `Q` did not
hold.

### Interaction with other features

- **`#[kani::should_panic]`** keeps its meaning: verification succeeds only if at least one check fails and every
  failed check is a panic. A panic that the clauses permit is not a failed check, so it does not count. A panic
  that they do not permit does.
- **`--prove-safety-only`** assumes that no panic occurs:
  - the clauses check nothing extra under it, and the replacement in step 2 never panics;
  - postconditions, including the `panics_if` check on return, are still checked.
- **Contracts checked as assertions.** When a function with a contract is called but is neither the function
  under verification nor replaced, Kani checks its contract as assertions around the call (unless
  `--no-assert-contracts` is passed). In that mode, the `panics_if` condition is checked on return, and the
  clauses do not permit panics: the panics of the callee are panics of the caller.
- **`autoharness`** generates `proof_for_contract` harnesses for functions with contracts, and those harnesses
  check the clauses like any other.
- **Multiple harnesses** for the same function all check the clauses, as they do for `requires` and `ensures`.

### Errors

- A second clause of the same kind on a function is a compile error.
- A function with these clauses and no `proof_for_contract` harness is reported like any other contract without
  a harness.
- If a condition refers to the result or is not a Boolean expression, that is a compile error, as for `requires`.

### What counts as a panic

A panic is any path that reaches Rust's panic machinery as Kani models it. That covers:
- `panic!`, `assert!`, `assert_eq!`, `assert_ne!`, `debug_assert!` and their variants, `unreachable!`, `todo!` and
  `unimplemented!`;
- library functions that panic, such as `Option::unwrap` and `Result::expect`;
- the panics rustc inserts for bounds checks, arithmetic overflow, division or remainder by zero, and invalid
  shifts;
- aborts, such as `std::process::abort`: Kani compiles code with `-C panic=abort`, and already reports an abort
  like a panic.

The following are not panics:
- undefined behavior (safety checks);
- Kani's own checks: `kani::assert`, `kani::check`, contract pre- and postconditions, and `kani::cover`;
- unsupported constructs.

As today, Kani compiles code with overflow checks and debug assertions enabled, so overflow and `debug_assert!`
failures are panics. See [Open questions](#open-questions) for builds where they are disabled.

### Output changes

Rust panics and Kani's own checks no longer share the `assertion` property class (see the
[prerequisite](#prerequisite-telling-panics-apart-from-kani-checks)). This changes some property names in Kani's
output, in `--export-json` and in SARIF. It does not change the description of any check, and it changes only
verification results that are wrong today:
- `should_panic` harnesses that pass only because a `kani::assert`, a postcondition or a precondition of a
  replaced contract failed now fail. In verify-rust-std, that affects the `should_fail_*` harnesses in
  `library/core/src/intrinsics/mod.rs`, which rely on a failed `stub_verified` precondition.
- `--prove-safety-only` now checks postconditions and `kani::assert`.

## Software Design

### Prerequisite: telling panics apart from Kani checks

Kani currently emits Rust panics and its own checks with the same property class, `assertion`:
- `codegen_panic` (the panic hook and `PanicStub`) uses it for panics;
- MIR `Assert` terminators use it for bounds, overflow and division checks;
- the `kani::assert` hook uses it for `kani::assert`. That hook also serves Kani's `assert!`, `assert_eq!` and
  `assert_ne!` overrides in `library/std`, and contract postconditions expand to `kani::assert`.

So neither the compiler nor the driver can tell a panic from a failed postcondition. Two existing features are
wrong because of this ([#5212](https://github.com/model-checking/kani/issues/5212)):
- `should_panic` accepts a failed postcondition or `kani::assert` as an expected panic.
- `--prove-safety-only` assumes away `assertion` checks, so it also assumes away postconditions and `kani::assert`.
  For example, a `proof_for_contract` harness verifies under `--prove-safety-only` even when the function
  violates its `ensures` clause.

The first step, which is useful on its own, is to give panics their own property class, `panic`:
- `codegen_panic`, the panicking MIR assertions and the `abort` intrinsic use `panic`.
- The `assert!`, `assert_eq!` and `assert_ne!` overrides call a new hidden function, `kani::panic_assert`, hooked
  to emit a `panic` check, instead of `kani::assert`. Automatic harness generation, which mines `assert!` guards
  in constructors, recognizes both functions.
- `kani::assert` and contract conditions keep `assertion`.
- The driver's `should_panic` classification, `--export-json`'s `failure_kind` and `--prove-safety-only` use
  `panic` instead of `assertion`.

### Encoding

The check needs a single verification run per harness.

1. **Library (`kani_core`).** Verifier-internal statics record, for the current call of the function under
   verification, the `panics_only_if` condition (if any), the `panics_if` condition, and the resulting
   permission: the `panics_only_if` condition if there is one, the `panics_if` condition otherwise. Three hidden
   functions update them: `set_panics_only_if(cond)` and `set_panics_if(cond)` record a condition and return the
   previous state, and `restore_panic_permission(prev)` restores it.
   - As for the existing contract-clause depth counter, the statics use `__CPROVER_`-prefixed symbol names and
     the functions `__VERIFIER`-prefixed ones, so that CBMC's contract instrumentation treats them as
     verifier-internal and does not check the updates against the write set of the function under contract.
   - The statics are reset at harness entry, together with the clause depth counter.
2. **Macro expansion (`kani_macros`).** Both clauses are parsed like `requires`.
   - In the check closure, each condition is evaluated after the preconditions are assumed and recorded with
     the matching `set_*` function. The previous state is restored when the function returns. `panics_if`
     also asserts, before returning, that its condition did not hold.
   - In the replace closure, each condition is evaluated after the preconditions are checked: `panics_if`
     panics if its condition holds, and `panics_only_if` panics nondeterministically if its condition holds.
     The panics go through `kani::panic`, so they are `panic` checks of the caller.
3. **Codegen (`kani-compiler`).** If a codegen unit contains the `set_*` functions, which happens only when the
   check closure of a function with one of these clauses is reachable, every `panic` check `assert(cond)` in the
   unit becomes `assert(cond || permitted)` followed by `assume(cond)`, where `permitted` is the recorded
   permission. The path ends at the panic either way, as today. Safety checks, postconditions and every other
   check are unchanged and are evaluated on all paths, so undefined behavior on the way to a permitted panic is
   reported. Other codegen units are unchanged.
4. **Driver (`kani-driver`).** No change beyond the `should_panic` classification above.

The permission is outside the function under verification `false`, since the statics are reset at harness entry
and restored on return, so panics in the harness itself, before or after the call, are reported as today. Panics
in callees of the function count as its own, since they propagate through it. A callee replaced by its contract
panics only as its own clauses say, and the permission of the function under verification then decides whether
that panic is permitted.

### Corner cases

- **Recursion.** Recursive calls are replaced by the contract, so step 2 of the replacement applies to them, and
  their panics are checked against the outer call's permission.
- **Closures and generic functions** passed to the function under verification are part of its body. A panic in
  a user-supplied comparator is a panic of the function, and is permitted only if the clauses allow it.
- **`kani::any_where` and `kani::assume` in the harness** constrain inputs as today, and the conditions are
  evaluated on the constrained inputs.
- **A panic while evaluating a condition** happens before the condition is recorded, so it is not permitted.
- **Loop contracts and quantifiers** are unaffected, because the permission is ordinary state.

## Rationale and alternatives

### A precondition-style `panics_if` (first version of #4230)

The first prototype in #4230 assumed `!cond` when checking the function and asserted `!cond` when replacing it.
That made the clause a precondition by another name:
- The inputs that satisfy `cond` were not verified at all. Undefined behavior on those paths, including before
  the panic, was hidden.
- Callers had to prove they never pass such inputs, which is not what a panicking API promises.

This RFC keeps all inputs in scope. It only changes whether reaching a panic, or returning, is a failure.

### Only a necessary condition

The first version of this RFC proposed only `panics_only_if`, and expressed "must panic" with a postcondition,
for example `ensures(|_| index <= old(self.len()))`. That works, but it is indirect, the failure does not say that
a panic was expected, and it is not the natural reading of a `# Panics` section.

remi-delmas-3000 suggested in #4230 to state both directions, a condition under which the function does not
panic and one under which it must, and to check that they are consistent (they do not overlap) and complete
(they cover all inputs, so the inputs are partitioned). This RFC follows that suggestion with the two clauses:
- `panics_only_if(C)` is the panic-freedom condition `!C`.
- `panics_if(Q)` is the panic condition.
- Consistency is checked implicitly: an input where `Q` holds but `C` does not can neither panic nor return, so
  it is reported.
- Completeness is built into `panics_if` alone, which permits panics exactly where it requires them. With both
  clauses, the inputs where `C` holds but `Q` does not are deliberately left open, for functions such as sorting
  with a user comparator whose panics cannot be stated exactly (see #3567).

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

`panics_if(cond)` reads like the `# Panics` sections of Rust documentation ("Panics if `index > len`"), and
states a sufficient condition: if `cond` holds, the function panics.

`panics_only_if(cond)` states a necessary condition: the function panics only if `cond` holds. Earlier names for
it were read the other way:
- `may_panic_if(cond)` reads as "if `cond`, the function may panic". It does not say that the function may not
  panic otherwise.
- `should_panic`'s "should" already means "must" in Kani.

Alternatives considered for `panics_only_if`:
- `no_panic_unless(cond)`, which says the same thing;
- its dual, `no_panic_if(!cond)`, which states the panic-free inputs instead of the panicking ones.

### Multiple clauses

Several `panics_only_if` clauses would have to be OR-ed, which is the opposite of how several `requires` or
`ensures` clauses combine. To avoid that confusion, a function has at most one clause of each kind, and users can
write `||`.

## Open questions

- **Builds without overflow checks or debug assertions.** In such builds, overflow wraps and `debug_assert!` does
  not run, so code after them executes, while Kani ends the path at the panic. This is also true of every Kani
  harness today. `--prove-safety-only` turns off debug assertions for this reason
  ([#4262](https://github.com/model-checking/kani/pull/4262)). A `panics_if` condition that relies on an
  overflow check, such as `NonZero::abs`'s, holds only with overflow checks enabled. Should the clauses offer, or
  default to, checking under both configurations?
- **Granularity.** Should users be able to permit only some kinds of panics, such as arithmetic overflow, or to
  match panic messages?
- **Mutable arguments.** The conditions are evaluated on entry. Is there a need to refer to the state at the time
  of the panic?

## Out of scope / Future Improvements

- **Panic safety.** Kani compiles code with `-C panic=abort` and does not explore unwinding. Drop glue and
  `catch_unwind` handlers that run during unwinding are therefore not checked.
  - A permitted panic says nothing about undefined behavior or broken invariants while unwinding. That is the
    concern for sorting with a panicking comparator
    ([#3567](https://github.com/model-checking/kani/issues/3567#issuecomment-2569213652),
    [verify-rust-std Challenge 8](https://github.com/model-checking/verify-rust-std/issues/56)).
  - Supporting it needs unwinding support in Kani and a postcondition on the panicking exit, comparable to SPARK's
    `Exceptional_Cases` aspect or JML's `signals` clauses.
- **Checking `# Panics` documentation** against the contract.
