# Soundness

This page documents Kani's soundness guarantees and known limitations.

## What Kani Checks

Kani automatically checks for the following classes of issues:

### Always enabled
- **Arithmetic overflow**: Addition, subtraction, multiplication, division,
  remainder, shift, and negation on integer types.
- **Division by zero**: Integer and floating-point division and remainder.
- **Pointer dereference validity**: Null pointers, dangling pointers, pointers
  to deallocated or dead objects, out-of-bounds pointers, misaligned pointers.
- **Array bounds**: Index out of bounds on arrays and slices.
- **Panics**: `assert!`, `unwrap()`, `expect()`, `panic!()`, `unreachable!()`,
  and any other panic path in the code under verification.
- **Shift distance**: Negative shift amounts and shifts exceeding the bit width.
- **Float-to-integer conversion**: Non-finite float values (NaN, infinity)
  are rejected before conversion to integer types.
- **Memory leaks**: Dynamically allocated memory that is never freed (within
  the verification scope).

### Opt-in (unstable)
- **Valid value invariants** (`-Z valid-value-checks`): Checks that values meet
  their type's validity requirements (e.g., `bool` is 0 or 1, `char` is a valid
  Unicode scalar value, enum discriminants are in range). This is an unstable
  feature that may change.
- **Uninitialized memory** (`-Z uninit-checks`): Checks that memory is
  initialized before being read through pointers. This is an unstable feature
  that may change.

## What Kani Does NOT Check

The following classes of undefined behavior are **not detected** by Kani:

- **Data races**: Kani verifies sequential code only. Concurrent programs are
  not supported.
- **Pointer aliasing violations** (Stacked Borrows / Tree Borrows): Kani does
  not track reference lifetimes or enforce Rust's aliasing rules. If aliasing
  violations cause a memory safety or assertion failure, Kani will detect the
  symptom but not the root cause.
- **Mutation of immutable data**: Same as aliasing — detected only if it causes
  an observable failure.
- **Incorrect use of inline assembly**: Kani does not support inline assembly.
  Global assembly (`global_asm!`) is ignored with a warning.
- **ABI violations**: Kani relies on `rustc` for ABI checking.
- **Transmute to invalid values**: `kani::any()` always produces valid values,
  but `transmute` of invalid bit patterns is not detected unless the unstable
  `-Z valid-value-checks` flag is enabled.

## Soundness Caveats

### CBMC backend
Kani uses [CBMC](https://github.com/diffblue/cbmc) as its verification backend.
CBMC is a mature tool but, like any complex software, may contain bugs. Kani
pins a specific CBMC version and runs CBMC's regression suite as part of its CI.

### Bounded verification
Kani performs bounded model checking. Loops are unwound up to a configurable
bound (`--default-unwind`). If the bound is insufficient, Kani emits an
unwinding assertion failure. Verification results are sound only if all
unwinding assertions pass.

### Floating-point arithmetic
CBMC's floating-point reasoning uses bit-precise IEEE 754 semantics for the SAT
backend. Results are sound for standard floating-point operations. However, some
platform-specific floating-point behavior (e.g., x87 extended precision) may
differ from the verification model.

### Object size limits
CBMC represents pointers with a fixed number of object bits (default: 16,
configurable via `--cbmc-args --object-bits N`). Programs that allocate more
objects than `2^N` may exhibit incorrect wrapping behavior. This is a known
limitation tracked in [#1150](https://github.com/model-checking/kani/issues/1150).

### Function pointers
By default, unresolved function pointers are modeled as nondeterministic calls
to any function with a compatible signature. This is sound but may be imprecise
(reporting spurious failures). An unstable `--restrict-vtable` flag limits
dispatch targets based on vtable analysis, but this feature is experimental and
may be imprecise in the other direction (missing valid targets).

## Reporting Soundness Issues

If you believe Kani has failed to detect a genuine issue (false negative),
please file a bug report with the `[F] Soundness` label at the
[soundness issues tracker](https://github.com/model-checking/kani/issues?q=label%3A%22%5BF%5D+Soundness%22).
