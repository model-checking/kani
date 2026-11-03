- **Feature Name:** Aliasing checks (`aliasing-checks`)
- **Feature Request Issue:** [#314](https://github.com/model-checking/kani/issues/314)
- **RFC PR:** [#5111](https://github.com/model-checking/kani/pull/5111)
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** branch [`aliasing-core`](https://github.com/feliperodri/kani/tree/aliasing-core)

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
  with a message that says so.

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
`autoharness`. No library API or attribute changes. Existing harnesses work as they are.

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

**The monitor is proved against Tree Borrows' rules.** The monitor's transition functions are
plain Rust in a macro that a Verus crate also expands, so the proof is about the code Kani
compiles. The proof has 126 verified obligations and no failures. Its theorems state that each
operation's result and new state refine the Tree Borrows rules on the abstract tree. Those
operations are: access, reborrow, deallocation, exposure, wildcard access and function return.
Verus isn't needed to build or use Kani. Running the proof in CI would add Verus as a CI
dependency; the proof-of-concept doesn't do that yet.

**CBMC dependency.** Shadow tags need a CBMC simplification that folds casts of integer pointer
constants ([diffblue/cbmc#9238](https://github.com/diffblue/cbmc/pull/9238)). Without it, values
read from shadow memory stay symbolic, and many harnesses run out of memory. The change is merged
in CBMC but not released. This feature should merge only once Kani's CBMC pin moves to a release
that contains it.

**Small changes elsewhere:**
- `kani-driver` keeps CBMC's shadow-memory field declarations bodiless when it generates bodies
  for missing functions, and passes the flag through.
- `kani_metadata` gets the new unstable feature.
- The proc macros don't change.

### Current results

All numbers are from the proof-of-concept, with CBMC built with #9238, at most 12 GB and 900 s per
harness, two jobs.

- **Miri's own borrow tests** (165 files from Miri's `tests/{pass,fail}/*_borrows`, ported so that
  Kani and Miri run the same file):
  - 77 tests where both report UB;
  - 57 where both pass;
  - none where Kani passes and Miri reports UB, and none the other way round;
  - 19 inconclusive: function pointers, `dyn`, transmutes through bytes, and pointers copied by
    multi-element memory intrinsics;
  - the rest use threads or are compile errors.
- **verify-rust-std challenges**, with the checks enabled on the existing harnesses:

  | Challenge | Verified | Not verified | Time with checks (without) |
  |---|---|---|---|
  | 9 (`Duration`) | 24/24 | none | 18 min (1 min) |
  | 11 (floats and ints) | 292/292 | none | 2.8 h (8 min) |
  | 14 (numeric conversions) | 190/190 | none | 1.4 h (5 min) |
  | 6 (`NonNull`) | 90/96 | 3 real UB, 2 timeouts, 1 inconclusive (`&dyn` call) | 4.5 h (18 min) |
  | 13 (`CStr`) | 0/14 | all out of memory | (4 min) |

  Challenges 11 and 14 ran on an earlier build of the proof-of-concept, without its latest cost
  fixes.

  The 3 real UB reports in Challenge 6 came from the harnesses, not from `NonNull`. Miri confirmed
  them, and the cause was fixed in Kani's `PointerGenerator` (#5108).

The cost comes mainly from symbolic tag counts: when program paths that created different numbers
of references merge, every later monitor operation works on a symbolic count. Challenge 13's
harnesses, which loop over nondeterministic-length strings, are the worst case we have found.

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

**Not doing this.** Kani keeps reporting harnesses with aliasing undefined behavior as verified.
Its users write and audit `unsafe` code, and they have no way to find these bugs except Miri on
concrete inputs.

## Open questions

- Should inconclusive aliasing results be reported as a third outcome (`UNDETERMINED`) instead of
  a failure? This affects other checks too, for example unresolved checks after CBMC runs out of
  memory, which are shown as failed today.
- Are the costs acceptable for an unstable feature, or should the cost work on symbolic tag counts
  (Challenge 13) come first? Users can enable the check per harness or per run.
- `&self` method calls through `&dyn Trait` are inconclusive today. They are common in real code.
  Should carrying the receiver's tag through vtable calls be required before the first release?
- How should the CBMC dependency be handled if a CBMC release with #9238 takes long?

## Out of scope / Future Improvements

- Tags through function pointers and `&dyn` method calls.
- Tags moved by multi-element copies of pointer-holding values.
- Weak protectors (`Box` arguments), the implicit access when a protector ends, and deallocation
  as a write access.
- Stacked Borrows as an alternative model.
- Lowering the cost of symbolic tag counts after loops with nondeterministic trip counts.
- Applying the check to the crates.io ecosystem with `autoharness`; an experiment on the top 50
  crates is planned.
