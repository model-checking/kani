- **Feature Name:** Aliasing checks (`aliasing-checks`)
- **Feature Request Issue:** [#314](https://github.com/model-checking/kani/issues/314)
- **RFC PR:** [#5111](https://github.com/model-checking/kani/pull/5111)
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** branch [`aliasing-core`](https://github.com/feliperodri/kani/tree/aliasing-core); contracts support on [`aliasing-contracts`](https://github.com/feliperodri/kani/tree/aliasing-contracts)

-------------------

## Summary

A new check, `-Z aliasing-checks`, reports memory accesses that break Rust's aliasing rules, as
defined by Tree Borrows, the aliasing model Miri implements. A harness that verifies with the
check enabled has no execution that violates those rules, or Kani reports the access it could not
check.

## User Impact

Kani's main audience writes or audits `unsafe` code, and today Kani misses a whole class of
undefined behavior there. Code that writes through a `*mut T` while a `&T` to the same memory is
alive, or uses a reference after another path invalidated it, verifies today, even though it is
undefined behavior. The compiler optimizes based on these rules, so such code can miscompile.
Miri finds these bugs, but only on the inputs a test runs. Kani can check them for all inputs a
harness covers.

What users get:
- the aliasing rules checked on every execution a harness covers, alongside Kani's existing
  memory-safety checks;
- harnesses that already exist, such as those of the verify-rust-std challenges, check aliasing
  with one extra flag;
- a check that never reports success for an access it could not follow. It gives up instead,
  with a message that says so;
- the check works with loop and function contracts, so unbounded loops and recursive functions
  can be verified free of aliasing violations, not only up to an unwinding bound.

The downsides:
- **Cost:** the checks are expensive. On the verify-rust-std challenges we ran, harnesses take 10
  to 20 times longer with the checks than without, and a few run out of memory (see "Current
  results" below).
- **Inconclusive results:** some programs can't be checked yet, for example `&self` method calls
  through `&dyn Trait`. Those results are inconclusive, so users will see failures that aren't
  bugs and will need to read the message to tell them apart.
- **Only Tree Borrows:** the rules are Tree Borrows'. Programs that only Stacked Borrows rejects
  pass. Rust has not adopted an official aliasing model, and a future official model may differ
  from both.

## User Experience

The check is enabled with `-Z aliasing-checks`, for `kani`, `cargo kani`, `kani verify-std` and
`autoharness`. No library API or attribute changes. Existing harnesses work as they are, including
those with loop contracts and function contracts: users don't write anything about tags or
permissions in invariants, `requires`, `ensures` or `modifies`.

```rust
#[kani::proof]
fn check() {
    let mut x = 0;
    let p = &mut x as *mut i32;
    let r = unsafe { &mut *p };
    unsafe { *p = 1 }; // The write through `p` invalidates `r`...
    *r = 2;            // ...so writing through `r` is undefined behavior.
}
```

```
Failed Checks: Undefined Behavior: memory accessed through a reference or pointer whose borrow was invalidated by a conflicting access (aliasing checks)
```

Each kind of violation has its own message, ending with "(aliasing checks)":
- an access through an invalidated reference or pointer;
- a write through a pointer that only has read permission, derived from a shared reference or
  from a mutable reference frozen by a later read;
- a write that invalidates a reference argument of a function that is still running (protector);
- deallocating memory that a reference argument of a running function points to;
- an access through a pointer created from an integer that no exposed pointer allows.

When the check cannot tell which reference a pointer comes from, the access fails with:

```
Failed Checks: Kani's aliasing checks do not know which reference this pointer was derived from, so they cannot check this access
```

This result is inconclusive: it is not a proof of undefined behavior. It is reported as a failure
so that a successful verification never depends on an access the check skipped. The user
documentation lists the constructs that lead to it.

Like `-Z uninit-checks`, the check tracks the state of one byte of memory per execution, chosen
nondeterministically, so if any byte is accessed against the rules, some execution tracks it and
the check fails. The trace shows that execution.

## Software Design

The proof-of-concept implements the design below. It is described briefly here; the details are in
the branch and its documentation (`docs/src/reference/experimental/aliasing-checks.md`).

**Instrumentation** (`kani-compiler`, a new MIR transformation pass,
`kani_middle/transform/check_aliasing.rs`). Every reference the program creates gets a *tag*: a
small integer held in a ghost local next to the pointer. Raw pointers keep their parent's tag.
The pass inserts calls to a library model for:
- every memory access (reads, writes, and the memory intrinsics);
- every reborrow;
- function entry and return (reference arguments are protected while the function runs);
- deallocation;
- exposing a pointer's address, and creating a pointer from an integer.

Tags travel across direct calls through a small call-state buffer. Accesses by calls to
foreign (C) functions are checked for the C functions Kani models (`memcpy`, `memset`, `strlen`,
...); any other foreign call that receives a pointer is reported as unsupported.

**Tags of pointers in memory** (*shadow tags*). When a pointer is stored in memory, its tag is
stored in CBMC's shadow memory, and it is read back when the pointer is loaded. Writes the
instrumentation can't see make the stored tags stale; those writes advance a generation counter
that invalidates the stored tags. Examples are loop contracts' havoc and calls to C functions.

**The monitor** (`library/kani_core`, the `kani_aliasing!` macros). The model tracks, for the one
tracked byte, up to 64 live tags. For each tag it keeps its ancestors and its Tree Borrows
permission (reserved, active, frozen or disabled, with the conflicted and interior-mutable
variants), as bit masks. Each step is a few bitwise operations, which keeps symbolic execution
cheap. A reborrow past 64 live tags gets the unknown tag, so the bound can make a result
inconclusive but can't make it pass.

Tags are dropped once nothing can reach them, so that loops don't run into the bound: a returning
function drops the tags it created, except the ones it returns, stores in memory or exposes; and a
reference that nothing else holds the tag of (assigned once by a reborrow, and otherwise only used
through a dereference) has its tag dropped when its storage ends. Only the most recent tag is
dropped, so a tag with a live child stays.

**Code that can only panic** is not checked: a standard-library function that never returns, other
than `exit` and `abort`, runs with the monitor off, and so do its callees. These are the functions
that build a panic message on the way to the panic. The execution fails at that panic anyway, so
skipping them can't turn a failure into a success; it only hides undefined behavior inside panic
formatting. Harnesses with `#[kani::should_panic]` keep the checks there, since an expected panic
isn't a failure. On the Challenge 13 reduction below, this halves the symbolic execution.

**The monitor is proved against Tree Borrows' rules.** The monitor's transition functions are
plain Rust in a macro that a Verus crate also expands, so the proof is about the code Kani
compiles. The proof has 163 verified obligations and no failures. Its theorems state that each
operation's result and new state refine the Tree Borrows rules on the abstract tree. Those
operations are: access, reborrow, deallocation, exposure, wildcard access and function return.
Verus isn't needed to build or use Kani. Running the proof in CI would add Verus as a CI
dependency; the proof-of-concept doesn't do that yet.

**Loop contracts.** A loop contract replaces the loop with one arbitrary iteration: CBMC havocs
what the loop may write, assumes the invariant, runs the body once and checks the invariant again.
The borrow state is part of the program state, so the instrumentation must say which borrow states
an arbitrary iteration can start from. Havocking it completely would report spurious violations
on every reference used after the loop. Keeping it as at loop entry would be unsound: a reference
that the first iteration invalidates would still look valid in the second.

The check uses the fact that Tree Borrows permissions only move one way. A tag's permission moves
along reserved, active, frozen, disabled, possibly skipping steps but never back, and its
ancestors never change. So at the loop head,
the instrumentation snapshots the monitor on entry and havocs it to any state where:
- every tag that existed on entry still has the same ancestors;
- its permission is the one on entry or a later one in that order.

Every monitor operation keeps this relation, so it holds after any number of iterations of any
loop body. It is an invariant the check generates itself, with no user annotation. The Verus proof
covers it in three parts: every monitor operation keeps the relation, including the access roots
below; dropping the tags of earlier iterations keeps the state well formed; and the havoc in the
monitor's code can produce every state the relation allows, up to the abstraction, and only
well-formed ones. What the proof takes from the instrumentation, unproved: the loop's accesses
act through descendants of its roots, and no access acts through a tag of an earlier iteration.

That relation alone loses precision: a reference the loop never uses would also be havocked to
"maybe disabled". The instrumentation refines it with the loop's *access roots*: tags that are
ancestors of every tag the loop accesses through. Every access in the loop is a child access for
them, so they stay valid across iterations. This is what lets a loop write through a reborrow of
`&mut x` and the code use `x` after the loop.

Three smaller pieces complete the loop abstraction:
- Tags created by earlier iterations are not kept. The tag locals of pointers the loop assigns are
  set to the unknown tag at the head, so a reference an earlier iteration created is inconclusive
  if used, never unchecked. The pass also adds those tag locals to the loop's write set, so CBMC
  never havocs one into another reference's tag.
- Shadow tags of memory the loop may write are invalidated at the head.
- The invariant itself is evaluated in *spec mode*: monitor operations are no-ops while it runs,
  so reading through a reference in an invariant doesn't change the borrow state.

**Function contracts.** `proof_for_contract` checks the function with the aliasing checks, as any
harness. A call replaced by its contract (`stub_verified`) is summarized with what Tree Borrows
already provides, without new contract clauses:
- reference arguments are retagged and protected for the call, as for a real call;
- each `modifies` target is havocked through the argument's tag, as a write access, so the
  replaced call invalidates the references a real write would. Pointers stored in the target lose
  their shadow tags;
- the `requires` and `ensures` clauses run in spec mode.

The summary relies on one assumption, which the callee's own `proof_for_contract` does not
check: the callee accesses the caller's memory only through pointers derived from its arguments.
`modifies` checks which memory the callee writes, but not through which pointer. Globals and
exposed pointers are where this assumption can fail.
Recursive functions work the same way: the recursive call inside `proof_for_contract` is replaced
by the contract.

**CBMC dependency.** Shadow tags need a CBMC simplification that folds casts of integer pointer
constants ([diffblue/cbmc#9238](https://github.com/diffblue/cbmc/pull/9238)). Without it, values
read from shadow memory stay symbolic, and many harnesses run out of memory. The change is merged
in CBMC but not released. This feature should merge only once Kani's CBMC pin moves to a release
that contains it.

Loop contracts need a second change,
[diffblue/cbmc#9392](https://github.com/diffblue/cbmc/pull/9392) (open). To infer a loop's write
set, CBMC's contracts instrumentation inlines every call in the harness, which now includes the
monitor: on a trivial loop it ran out of 16 GB. With the change, the inference inlines a summary of
what each monitor function writes instead of its body, and the same loop takes 2.2 GB and 10 s.
Without it, aliasing checks and loop contracts can't be combined. CBMC also leaves out of the
inferred write set writes through pointers loaded from memory
([diffblue/cbmc#9393](https://github.com/diffblue/cbmc/issues/9393)). This is unsound for
functions it doesn't instrument, such as the monitor's. The borrow state doesn't rely on CBMC's
havoc, since the loop head replaces it (above). Whether other state the monitor's functions write
through such pointers is affected has not been checked yet.

**Small changes elsewhere:**
- `kani-driver` keeps CBMC's shadow-memory field declarations bodiless when it generates bodies
  for missing functions, and passes the flag through.
- `kani_metadata` gets the new unstable feature.
- The proc macros change in two places: the loop-invariant closure is marked, so the pass doesn't
  instrument it, and the guard around a contract clause ends with the clause.
- Loop-contract harnesses keep the monitor's statics deterministic (CBMC's contracts
  instrumentation makes every static nondeterministic by default).

### Current results

All numbers are from the proof-of-concept, with CBMC built with #9238, at most 12 GB and 900 s per
harness, two jobs.

- **Miri's own borrow tests** (165 files from Miri's `tests/{pass,fail}/*_borrows`, ported so that
  Kani and Miri run the same file):
  - 76 tests where both report UB;
  - 57 where both pass;
  - none where Kani passes and Miri reports UB, and none the other way round;
  - 1 excluded, `pass/tree_borrows/cell-inside-struct`: it runs Miri with
    `-Zmiri-tree-borrows-no-precise-interior-mut`, a mode Kani doesn't implement (Kani always
    tracks interior mutability per byte, and reports UB there). An earlier count included it
    because our runner dropped that flag;
  - 19 inconclusive: function pointers, `dyn`, transmutes through bytes, and pointers copied by
    multi-element memory intrinsics;
  - the rest use threads or are compile errors.
- **verify-rust-std challenges**, with the checks enabled on the existing harnesses:

  | Challenge | Verified | Not verified | Time with checks (without) |
  |---|---|---|---|
  | 9 (`Duration`) | 24/24 | none | 18 min (1 min) |
  | 11 (floats and ints) | 292/292 | none | 2.8 h (8 min) |
  | 14 (numeric conversions) | 190/190 | none | 1.4 h (5 min) |
  | 6 (`NonNull`) | 93/96 | 2 timeouts, 1 inconclusive (`&dyn` call) | 4.5 h (18 min) |
  | 13 (`CStr`) | 0/14 | all out of memory | (4 min) |

  Challenges 11 and 14 ran on an earlier build of the proof-of-concept, without its latest cost
  fixes.

  Challenge 6 first had 3 UB reports. They came from the harnesses, not from `NonNull`: Miri
  confirmed them, the cause was in Kani's `PointerGenerator`, fixed in #5108, and the 3 harnesses
  now verify.

- **Contracts**, on small programs where Miri runs the unrolled loop or the concrete call:
  - 10 loops with contracts, 5 with UB and 5 without, including UB that only happens in a later
    iteration, a reborrow created in the loop, and a loop with a nondeterministic trip count
    (4 s). Kani and Miri agree on all 10. Before the loop abstraction, the UB in a later iteration
    verified.
  - 7 function-contract harnesses, `stub_verified` and recursive `proof_for_contract`, 4 with UB.
    Kani and Miri agree on all 7. The recursive UB case needs about 10 GB.
  - Kani's existing loop-contract and function-contract test suites pass with the change.

**Cost.** A profile of the Challenge 13 reduction (a nul-terminated view of a nondeterministic
slice, and a loop over its bytes) showed where the time goes. 63% of the symbolic execution was in
code that only builds panic messages, and most of the rest in per-call overhead: every function
return runs the monitor, and every library entry point looks up its state. Symbolic tag counts, the
cause we had assumed, contribute little: a variant whose returned reference doesn't depend on the
iteration costs the same, relative to the program without checks. With the changes above (no
checks in panic-only code, a fast path for returns that drop nothing, tags dropped when their
storage ends):

| Challenge 13 reduction | Before | After |
|---|---|---|
| 3 bytes | 122 s, 1.9 GB | 74 s, 1.2 GB |
| 12 bytes | 767 s, 4.4 GB | 220 s, 3.0 GB |

Without the checks, both take under 10 s. The full Challenge 13 harnesses still have to be rerun.

## Rationale and alternatives

**Why Tree Borrows, not Stacked Borrows.** Tree Borrows is Miri's newer model, and it accepts real
code that Stacked Borrows rejects. For example, a `&mut` that is never written survives foreign
reads, and a raw pointer from a reference to one element of an array may access the rest of the
array. Its state is a tree of tags with a permission each, which maps onto
fixed-size bit masks. Stacked Borrows' per-location stack doesn't map as well. An earlier prototype
of Stacked Borrows in Kani ([#3475](https://github.com/model-checking/kani/issues/3475), branch
`features/stacked_borrows`) didn't reach the point of handling function calls and pointers in
memory. Supporting Stacked Borrows as a second model later remains possible.

**Why track one byte.** Tracking every byte's tree would put the whole heap's aliasing state into
the symbolic program. One nondeterministically chosen byte, as `-Z uninit-checks` does, keeps the
state small and finds a violation at any byte, because some execution tracks that byte. The cost
is that the trace shows one byte, and that per-byte state can't be compared across bytes.

**Why inconclusive results fail.** Treating an unknown tag as "anything goes" would make the check
unsound. Treating it as an error makes every failure a candidate bug that users must triage. We
chose to fail with a distinct message. An alternative is a new verification result,
`UNDETERMINED`, next to `SUCCESS` and `FAILURE`, which needs changes in Kani's result reporting.
That's an open question below.

**Why a proof, and why it isn't enough.** The monitor encodes subtle rules: protectors,
conflicted reserved, interior mutability, wildcard accesses. The Verus proof makes the monitor's
code correct with respect to its specification, independently of the tests. It can't show that
the specification is Tree Borrows. In one case we proved the monitor against a specification that
got the protected reserved transition wrong, and Miri's test corpus caught it.

So correctness rests on both:
- the proof, for the code against the specification;
- differential testing against Miri, for the specification and the instrumentation.

The early version had 26 Miri tests where Kani passed and Miri reported UB. All are fixed, and
the current version has none.

**Symbolic execution path by path.** Soteria Rust
([Ayoun, Sjöstedt, Raad, PLDI 2026](https://doi.org/10.1145/3808306)) checks Tree Borrows by
exploring one path at a time with a concrete tree per allocation. We ran it, with Tree Borrows on,
on the same files:

| | Miri borrow tests (165): Soteria | Kani | Kani's aliasing tests (76): Soteria | Kani |
|---|---|---|---|---|
| both report UB | 52 | 76 | 28 | 35 |
| both pass | 59 | 57 | 38 | 34 |
| missed UB | 38 | 0 | 5 | 0 |
| false alarm | 3 | 0 | 0 | 0 |
| inconclusive, unsupported or excluded | 13 | 32 | 5 | 7 |

Soteria is much faster on these tests, 0.3 s against 5.4 s on the median Miri test, because each
is a single concrete path. Most of its missed UB come from pointers created from integers, which
it doesn't check; the rest from rules it departs from (retags without a read, deallocation as a
read, no access when a protector ends, interior mutability per type). It has no loop invariants
or function contracts, so it can't verify a loop with a nondeterministic trip count, which Kani
does with a loop contract. Its per-path state needs no symbolic byte, but a symbolic byte gives
the same coverage: Tree Borrows state is independent per byte, and some execution tracks the byte
where a violation happens.

**Not doing this.** Kani keeps reporting harnesses with aliasing undefined behavior as verified.
Its users write and audit `unsafe` code, and they have no way to find these bugs except Miri on
concrete inputs.

## Open questions

Each question has a proposal; reviewers' input is welcome on all of them.

- **Should inconclusive results be a third outcome (`UNDETERMINED`)?** It would also help other
  checks, for example the checks left unresolved when CBMC runs out of memory, which are shown as
  failed today. *Proposal:* keep failing with a distinct message for the first release, and
  propose `UNDETERMINED` separately, since it changes Kani's result reporting for every check.
- **Is the cost acceptable for an unstable feature?** Harnesses take 10 to 20 times longer, and
  Challenge 13 still has to be rerun with the cost fixes. *Proposal:* yes. The check is opt-in per
  run, and verified results stay sound when the cost is high; the cost work continues after the
  first PRs.
- **Must `&self` calls through `&dyn Trait` work before a first release?** They are inconclusive
  today, never unchecked. *Proposal:* no; carry the receiver's tag through vtable calls in a
  follow-up.
- **How should the CBMC dependency be handled?** *Proposal:* merge the parts that don't need it
  first (see "Implementation plan"), and the instrumentation once Kani's CBMC pin moves to a
  release with #9238; loop contracts with aliasing checks additionally need #9392.
- **Is the automatic loop invariant precise enough?** It keeps access roots valid but forgets the
  exact permission of every other tag the loop touches, and users can't state more yet.
  *Proposal:* measure it on the loops of the verify-rust-std challenges before adding predicates
  for invariants.

## Implementation plan

The proof-of-concept is about 11,000 lines, most of them tests and the Verus proof. It would land
as a series of PRs, each reviewable and testable on its own:

1. **The monitor and its Verus proof** (`library/kani_core/src/aliasing_monitor.rs`,
   `library/kani_core/verus`), with a CI job that runs the proof. Nothing calls the monitor yet.
   Opened as [#5120](https://github.com/model-checking/kani/pull/5120).
2. **The instrumentation** (`-Z aliasing-checks`): the MIR pass, the library wrappers, the driver
   changes, the user documentation and the tests. It needs a CBMC release with #9238.
3. **Contracts:** loop contracts (the loop-head abstraction, spec mode) and function contracts.
   Loop contracts need CBMC #9392.
4. **Follow-ups**, each with an issue: tags through `&dyn` and function pointers, multi-element
   copies, `UNDETERMINED`.

## Out of scope / Future Improvements

- Tags through function pointers and `&dyn` method calls.
- Tags moved by multi-element copies of pointer-holding values.
- Stacked Borrows as an alternative model.
- Predicates for loop invariants and contracts such as `can_read(r)` or `can_write(r)`, for loops
  where the automatic invariant is too coarse. Permissions only move one way, so "at least this
  permission" facts survive induction.
- Loops whose write set CBMC infers (no `loop_modifies`) report spurious dereference failures
  along with the real result; explicit `loop_modifies` avoids them.
- Further cost work: the per-call overhead of the monitor, and symbolic tag counts where they
  matter.
- Reclaiming tags that a function can no longer reach before it returns, beyond the cases above,
  to raise the 64-tag bound in long-running loops.
- Applying the check to the crates.io ecosystem with `autoharness`; an experiment on the top 50
  crates is planned.
