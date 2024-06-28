- **Feature Name:** Rust UB Checks (`rust-ub-checks`)
- **Feature Request Issue:** [#3089](https://github.com/model-checking/kani/issues/3089)
- **RFC PR:** [#XXXX](https://github.com/model-checking/kani/pull/3091)
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:**

-------------------

## Summary

Specify a consistent framework for Undefined Behavior (UB) checks in Kani.

## User Impact

Provide a consistent user experience for all UB checks in Kani, while establishing the best mechanisms for
further instrumentation of undefined behavior in Kani.

## User Experience

A UB check should behave consistently independent on how and where it is implemented.
The detection of an undefined behavior should impact the status of any assertion that may be reachable from the
detected undefined behavior.

For simplicity, we propose that all undefined behavior checks are modelled as assert-assume pairs.
We propose that the status of all passing assertions that may be affected by this check to be displayed as
UNDETERMINED if one or more UB checks fail. [^all-passing]

All failing assertions should have their status preserved, unless the UB check is implemented using "demonic"
non-determinism, such as CBMC's deallocated check.
See [0010-rust-ub-checks.md#open-questions] for alternatives to be considered.

### Concrete playback of UB checks

A counter example that triggers UB may require extra tooling for debugging, e.g.: MIRI or valgrind.
We propose that tests added by concrete playback in those cases should include a comment stating such limitation.
Other possibilities are discussed in [0010-rust-ub-checks.md#open-questions].

### Delayed undefined behavior

In some cases, undefined behavior may occur in safe code that interacts with unsafe code.

For example, the code below shows how reading a boolean value in safe code may trigger UB due to an
invalid write:

```rust
#[kani::proof]
fn buggy_code() {
    let mut value: bool = false;
    unsafe {
        let ptr: *mut u8 = &mut value as *mut _ as *mut u8;
        *ptr = 8; // This assignment does not cause UB...
    }
    assert!(value); // ...but this read triggers UB! ⚠️
}
```

It may not be scalable for Kani to verify that every value produced is valid.
In cases like that, we may opt for implementing one of the following UB checks:

1. Perform static analysis to first identify which code may be affected by unsafe code, and only instrument those that
   are reachable from the unsafe code.
2. Add eager UB detection in unsafe code that will fail verification if there are side effects that we may not be
   able to detect efficiently. For example, we could fail if casting between types that have different value
   requirements, i.e.: fail in the cast `*mut bool as *mut u8`.

For option 2, Kani may fail verification even if the code does not have UB.
In those cases, Kani check must be explicit about this failure being an over-approximation.
For that, we propose that the check message is clear about this limitation, and potentially its status
(see [0010-rust-ub-checks.md#open-questions]).

### Opt-out a class of UB checks

Undefined behavior checks that have been stabilized should be added by default by Kani.
However, Kani should provide a fine-grained control for users to choose which checks to disable.

For every class of UB checks to be added, we shall add a command line argument to disable the check for all harnesses,
as well as an attribute that would allow users to opt-out a check for specific harnesses.

Command line arguments should follow the existing name schema:
`--no-[CHECK-NAME]-checks` where the CHECK-NAME is kebab case. There is no need to add `--[CHECK-NAME]-checks` since
they are enabled by default.

We propose to add the following attribute:

```rust
#[kani::proof]
#[kani::ignore(CHECK_NAMES)]
fn harness() {}
```

Where `CHECK_NAMES` is a list of check category names in snake case, such as `memory_safety`, `valid_values`.

For checks that are unstable, Kani should support both opt-out mechanisms.
These checks shall be included by default if their respective unstable flag are passed, i.e., `-Z[CHECK-NAME]`

## Software Design

*To be filled in the next iteration*

## Rationale and alternatives

Kani already has undefined behavior checks, some are implemented on the CBMC side, such as access out bounds, and
arithmetic overflow.
While others are implemented on Kani's side, such as intrinsics validation checks, and some are a
result of instrumentation to the Rust standard library.

The behavior of these checks varies according to how they are added.
The ones on CBMC side do not interrupt the analysis of a path, while the ones on Kani / Rust std side will.

For example, memory bound checks are implemented in CBMC, and the following harness today will trigger 2 different
failures:

```rust
#[kani::proof]
pub fn is_zero() {
    let var = [0u32; 4];
    let ptr = &var as *const u32;
    let idx: usize = 4;
    assert_eq!(unsafe { ptr.add(idx).read() }, 0);
}
```

The failed checks are:

```plaintext
Failed Checks: assertion failed: unsafe { ptr.add(idx).read() } == 0
Failed Checks: dereference failure: pointer outside object bounds
```

While the following harness only fails the standard library check:

```rust
#[kani::proof]
pub fn is_zero() {
    let var = [0; 4];
    let idx: usize = 4;
    assert_eq!(unsafe { *var.get_unchecked(idx) }, 0);
}
```

Failing due to an assumption check inside `get_unchecked` function.

```plaintext
Failed Checks: assumption failed
```

We would like to keep taking advantage of existing instrumentation to maximize the UB check coverage.
The mechanism used to build the check, however, shouldn't influence in the user experience.

### Opt-out vs opt-in

The default behavior matches the one that we believe is the recommended for most users due to soundness.
In some cases, users may need to disable some checks for reasons such as spurious CEX or proof scalability.
Thus, we suggest that every category of checks should have an opt-out mechanism that can either be global or local;
i.e.: via command argument or harness attribute.

The main disadvantage of this model is that stabilizing a new check category is a non-backward compatible change.
I.e.: It may impact the status of a previously passing harness if a new check fails.

## Open questions

- Should we add a new status for potentially spurious counter examples? Such as:
    - Counter examples detected together with a non-deterministic UB check.
    - Over-approximating UB checks.
- How should UB checks affect concrete playback? Is a comment enough?
    - Another possibility would be to add a `unreachable!()` statement at the end of the test case with a message
      explaining why the statement should be unreachable.
- Should we deprecate the `--no-default-checks` and all `--[CHECK-NAME]-checks` checks.

## Out of scope / Future Improvements

- **List enabled checks:** Kani could include a command that list all the categories of undefined behaviors supported.

[^all-passing]: For simplicity, we currently over-approximate this by marking all passing assertions as UNDETERMINED.