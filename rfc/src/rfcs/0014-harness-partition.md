- **Feature Name:** Partitioned Proofs (`partition-proof`)
- **Feature Request Issue:** [#3006](https://github.com/model-checking/kani/issues/3006)
- **RFC PR:** https://github.com/model-checking/kani/pull/4228
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** None yet, the previous prototype uses an out of date API.

-------------------

## Summary

It can often be useful to subdivide an expensive proof harness so that different parts of the input space are verified separately.
This is also known as *proof by cases* or a *case split*.
Adding the built-in ability to partition a proof harness into different pieces (that each make differing assumptions about their inputs) could reduce the cost of expensive proofs, while allowing Kani to automatically check that the partitions cover the entire input space and, thus, will be equivalent to a single-harness proof.

## User Impact

Imagine that you have a function to verify like the following (based on the example from [#3006](https://github.com/model-checking/kani/issues/3006)).

```rust
pub fn target_fn(input: i32) -> isize {
    let val = if input > 0 {
        very_complex_fn_1(input)
    } else {
        very_complex_fn_2(input)
    };
    very_complex_fn_3(val)
}

#[kani::proof]
pub fn proof_harness() {
    let input = kani::any();
    assert!(target_fn(input) > 0)
}
```

Since there are two tricky to analyze function calls, but only one will ever be called on a given input, you might want to verify all values of `input` where `input > 0` that will take the first branch separately from those that will take the second.
This way, each verification run will only have to reason about two of the three complex function calls, and you could use Kani's parallel proof runner to run both proofs at once.

The best way to currently do this is by manually partitioning out these paths into two proof harnesses.

```rust
#[kani::proof]
pub fn first_branch_harness() {
    let input = kani::any_where(|i: &i32| *i > 0i32);
    assert!(target_fn(input) > 0)
}

#[kani::proof]
pub fn second_branch_harness() {
    let input = kani::any_where(|i: &i32| *i < 0i32); // ERROR: This should've been i <= 0
    assert!(target_fn(input) > 0)
}
```

One could also write preconditions for `very_complex_fn_{1_2}` that restrict `input` accordingly, write `proof_for_contract` harnesses for those, and then use `stub_verified` on the `target_fn` harness. 

However, either of these strategies:
- **can affect soundness**--there's no guarantee that your partitions will fully span the space of possible inputs.
The only way to determine that a set of proofs like the one above are incomplete (as it forgets to verify the value of 0 for `input`) is by manual inspection.
This gets infeasible for proofs with complex partition rules like those found in the [proofs for the standard library's unchecked multiplication](https://github.com/model-checking/verify-rust-std/blob/1c4ea17a99b9202f96608473083998b116bb6508/library/core/src/num/mod.rs#L1818-L1836).
- **increases user burden**--instead of having to write and maintain a single proof, the user now has to handle a proof for each partition.

Instead, Kani should provide a feature to automatically partition a proof based on certain conditions and ensure that the newly partitioned harnesses are equivalent to a single-harness proof.

## User Experience

Kani will introduce a new `kani::partition` function, gated behind an unstable feature flag.
When calling this function, the user would provide the type of the partition variable, as well as a set of partition conditions and a closure with the actual code to run on the partitioned variable (we'll call this the *partition closure*).

For example, the above would become

```rust
#[kani::proof]
pub fn partitioned_proof() {
    kani::partition::<i32, _, _>([|a| *a > 0, |a| *a < 0], |input: i32| assert!(target_fn(input) > 0));
}
```

Kani would then automatically handle generating the partitioned proofs checking that the conditions fully cover the domain of the partition variable.
Overlaps would be allowed as they don't affect soundness and may be useful in certain cases (see [below](#1-allowing-overlapping-partition-conditions) for more discussion).

When running such a partitioned proof, the user's output would be as shown below. Notice the separate proof harnesses generated for each partition and the additional one generated to prove that the partitions fully cover the domain of the partition variable.

```
Checking partition #1 (*input > 0) of harness partitioned_proof...
CBMC 6.7.1 (cbmc-6.7.1)
/* MORE CBMC OUTPUT */
RESULTS:
Check 1: partitioned_proof_partition_1.assertion.1
     - Status: SUCCESS
     - Description: "assertion failed: target_fn(input) > 0"
     - Location: src/pre_partition.rs:8:5 in function partitioned_proof_partition_1

Checking partition #2 (*input < 0) of harness partitioned_proof...
CBMC 6.7.1 (cbmc-6.7.1)
/* MORE CBMC OUTPUT */
RESULTS:
Check 1: partitioned_proof_partition_2.assertion.1
     - Status: SUCCESS
     - Description: "assertion failed: target_fn(input) > 0"
     - Location: src/pre_partition.rs:8:5 in function partitioned_proof_partition_2

Checking coverage of partitions for harness partitioned_proof...
CBMC 6.7.1 (cbmc-6.7.1)
/* MORE CBMC OUTPUT */
RESULTS:
Check 1: partitioned_proof_coverage.assertion.1
     - Status: FAILURE
     - Description: "partitions for partitioned_proof do not cover all possible values of the i32 type."
     - Location: src/pre_partition.rs:11:5 in function partitioned_proof_coverage
```

Note that the coverage check fails, since we're missing the `i = 0` case.

## Software Design

This change would introduce the new `kani::partition` function, with the following signature:

```rust
pub fn partition<T: Arbitrary, R, const N: usize>(
    conditions: [fn(&T) -> bool; N],
    and_run: fn(T) -> R, // the "partition closure"
) -> R
```

This signature makes a few key design decisions:
- *It takes in an array of `conditions` with the specific length `const N`*, ensuring the number of harnesses we have to generate is known at compile time when we're doing partition generation.
- *Partition conditions & the partition closure are represented by function pointers*.
This allows the user to provide closures of the same signature, but only if those closures do not capture any state.
Allowing conditions closures to capture runtime state like local variables would be unwise as partitions are conceptually static divisions of a proof which should not depend on runtime values.
- *The function returns the same type `R` as the underlying partition closure*, with the thought being that value could then be used in later computation without affecting the mechanics of a partition.
- *The partition variable is bound by `T: Arbitrary`*.
This would allow us to construct a new non-deterministic value with `kani::any()` that can then be passed to the partition closure.

When the `kani-compiler` encounters a call to this function, it will transparently replace the entire `#[kani::proof]` that called it with a set of new generated functions that, as a whole, will represent the partitioned proof. This new partitioned proof would contain:

1. one new proof harness for each partition

In the above example, this would be along the lines of:

```rust
#[kani::proof]
/// (auto-generated partition of `input` for condition `|a| *a > 0`)
pub fn partitioned_proof_partition_1() {
    /* OTHER CODE BEFORE THE CALL TO `kani::partition` IF THERE WAS ANY */
    let partition_variable = kani::any::<i32>();
    kani::assume((|a: &i32| *a > 0)(&partition_variable));
    (|input: i32| assert!(target_fn(input) > 0))(partition_variable);
}
```

2. an additional proof harness to check that the partitions fully cover the domain of the variable

This would be along the lines of:
```rust
pub fn partitioned_proof_coverage() {
    /* OTHER CODE BEFORE THE CALL TO `kani::partition` IF THERE WAS ANY */
    let partition_variable = kani::any::<i32>();

    // checks that any possible value of partition_variable will fall into at least one partition.
    let is_in_partitions = [|a: &i32| *a > 0, |a: &i32| *a < 0].into_iter().any(|cond| cond(&partition_variable));
    kani::assert(is_in_partitions, "partition conditions for partitioned_proof do not cover all possible values of a i32.");
}
```

### Corner cases
Under the type signature of `kani::partition`, the user can generate the partition conditions conditionally, e.g.:

```rust
#[kani::proof]
pub fn strange_partitioned_proof() {
    let f: fn(&i32) -> bool = if kani::any::<i32>() > 10 {
        |a: &i32| *a < 0
    } else {
        |a: &i32| *a < -2
    };

    kani::partition::<i32>([f, |a: &i32| *a < 0], ...);
}
```

The behavior of a partition in this case seems ill-defined, so Kani will detect cases where the partition conditions are not constant closures or function items and emit a compile error.
This would likely require new analysis from Kani to properly detect these cases.

## Rationale and alternatives

### 1. `kani::partition` as an attribute
We had initially considered implementing the partition as an attribute placed on a proof harness, similar to `kani::proof`.
However, we decided it would be best implemented as a function because a partition conceptually represents a split on variables in the proof, rather than something inherent to the proof itself.
Similarly, we wanted to maintain the convention that proof harnesses do not take in any inputs, and all attribute solutions would have to break that.

### 2. Allowing overlapping partition conditions
As a corollary to checking that partitions span the space of all possible inputs, I had initially considered giving a warning if partitions are overlapping as this could indicate ill-defined bounds.

However, this kind of overlap would not affect the correctness of the partition, as every possible value of the partition variable would still be checked, just potentially more than once.
Admittedly, it's fairly difficult to come up with a realistic use case in which an overlap is helpful for users, so whether this should be allowed is up for debate.

## Open questions

1. How can we ensure that type errors in the `kani::partition` function call are clear, understandable and actionable for users?
2. How can we test that there are no correctness issues introduced by our partition implementation?
3. Should verification fully panic if a partitioned proof is lacking full coverage or just have that one assertion fail verification?
4. What's the best way to provide support for partitioning `BoundedArbitrary` types?
Potentially through a separate function `partition_bounded` which would be generic over `T: BoundedArbitrary` and a `const N: usize` which would be used for the call to `kani::bounded_any()`.
5. How should users be able to control our coverage checks?
Users may only want to test a subset of the input space (e.g., only testing multiplication for certain integer ranges), but the current design mandates that they cover the whole input space.
We could potentially introduce a separate API, e.g. `unsafe_partition`, that allows this, or some other mechanism to skip the coverage check.
6. Should overlapping partition conditions be allowed?
7. (Similar to #3) How can a coverage check failure be best presented to users?
Perhaps by running the coverage harness with concrete playback to generate a test case that doesn't fall into the partitions?

[^unstable_feature]: This unique ident should be used to enable features proposed in the RFC using `-Z <ident>` until the feature has been stabilized.