- **Feature Name:** Partitioned Proofs (`partition-proof`)
- **Feature Request Issue:** [#3006](https://github.com/model-checking/kani/issues/3006)
- **RFC PR:** https://github.com/model-checking/kani/pull/4228
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** [prototype on local branch](https://github.com/model-checking/kani/compare/main...AlexanderPortland:kani:harness-partitioning)

-------------------

## Summary

It can often be useful to subdivide an expensive proof harness so that different parts of the input space are verified separately.
Adding the built-in ability to partition a proof harness into different pieces (that each make differing assumptions about their inputs) could reduce the cost of expensive proofs, while allowing us to automatically check that the partitions cover the entire input space and, thus, will not affect soundness.

## User Impact

Imagine that you have a function to verify like the following (based on the example from [#3006](https://github.com/model-checking/kani/issues/3006)).

```rust
pub fn target_fn(input: i32) -> isize {
    if input > 0 {
        hard_to_analyze_fn_1(input)
    } else {
        hard_to_analyze_fn_2(input)
    }
}

#[kani::proof]
pub fn proof_harness() {
    let input = kani::any();
    assert!(target_fn(input) > 0)
}
```

Since there are two tricky to analyze function calls, but only one will ever be called on a given input, you might want to verify all values of `input` where `input > 0` that will take the first branch separately from those that will take the second.
This way, each solve would be smaller in isolation, and you could use Kani's parallel proof runner to run both proofs at once.

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

However, this strategy:
- **can affect soundness**--there's no guarantee that your partitions will fully span the space of possible inputs.
The only way to determine that a set of proofs like the one above are incorrect (as it forgets to account for when `i == 0`) is by manual inspection, which gets infeasible for proofs with complex partition rules like those found in the [proofs for the standard library's unchecked multiplication](https://github.com/model-checking/verify-rust-std/blob/1c4ea17a99b9202f96608473083998b116bb6508/library/core/src/num/mod.rs#L1818-L1836).
- **increases user burden**--instead of having to write and maintain a single proof, the user now has to handle a proof for each partition.

Instead, Kani should provide a feature to specify partition conditions for a given harness, automatically checking that the partitioned harnesses cover the entire input space.

## User Experience

The current thought is for users to set a `-Z partition-proof` flag which will allow them to use a new `kani::partition` function call. 
In this call, they would provide the type of the partition variable to generate, as well as a set of partition conditions and a closure with the actual code to run on the partitioned variable (we'll call this the *partition closure*).

For example, the above would become

```rust
#[kani::proof]
pub fn partitioned_proof() {
    kani::partition::<i32>([|a| *a > 0, |a| *a < 0], |input: i32| target_fn(input));
}
```

Kani would automatically handle checking that the condition variables fully cover the domain of the partition variable and generating the partitioned proofs.
Overlaps would be allowed as they don't affect soundness and may be useful in certain cases (see [below](#2-checking-for-overlapping-partitions) for more discussion).

When running such a partitioned proof, the user's output would be as shown below. Notice the separate proof harnesses generated for each partition and the additional one generated to prove that the partitions fully cover the domain of the partition variable.

```
Checking partition #1 *input > 0 of harness partitioned_proof...
CBMC 6.7.1 (cbmc-6.7.1)
/* MORE CBMC OUTPUT */
RESULTS:
Check 1: partitioned_proof_partition_1.assertion.1
	 - Status: SUCCESS
	 - Description: "assertion failed: target_fn(input) > 0"
	 - Location: src/pre_partition.rs:8:5 in function partitioned_proof_partition_1
     - Partition: #1 - *input > 0

Checking partition #2 *input < 0 of harness partitioned_proof...
CBMC 6.7.1 (cbmc-6.7.1)
/* MORE CBMC OUTPUT */
RESULTS:
Check 1: partitioned_proof_partition_2.assertion.1
	 - Status: SUCCESS
	 - Description: "assertion failed: target_fn(input) > 0"
	 - Location: src/pre_partition.rs:8:5 in function partitioned_proof_partition_2
     - Partition: #2 - *input < 0

Checking coverage of partitions for harness partitioned_proof...
CBMC 6.7.1 (cbmc-6.7.1)
/* MORE CBMC OUTPUT */
RESULTS:
Check 1: partitioned_proof_coverage.assertion.1
	 - Status: FAILURE
	 - Description: "partitions for partitioned_proof do not cover all possible values of a i32."
	 - Location: src/pre_partition.rs:11:5 in function partitioned_proof_coverage
```

## Software Design

This change would introduce the new `kani::partition` function, with the following signature:

```rust
pub fn partition<T: Arbitrary, R, const N: usize>(
    conditions: [fn(&T) -> bool; N],
    and_run: fn(T) -> R, // the "partition closure"
) -> R
```

This signature makes a few key design decisions:
- *It takes in an array of `conditions` with the specific length `const N`*, ensuring the number of harnesses we have to generate is knowable at compile time when we're doing the generation.
- *Partition conditions & the partition closure are represented by function pointers*. 
This allows the user to provide closures of the same signature, but only if those closures do not capture any state.
- *The function returns the same type `R` as the underlying partition closure*, with the thought being that value could then be used in later computation.
- *The partition variable is bound by `T: Arbitrary`*. 
This would allow us to construct a new non-deterministic value with `kani::any()` that can then be passed to the partition closure.

When the `kani-compiler` encounters a call to this function, it will transparently replace the entire `#[kani::proof()]` that called it with a set of new generated functions that, as a whole, will represent the partitioned proof. This new partitioned proof would contain:

1. one new proof harness for each partition

In the above example, this would be along the lines of:

```rust
#[kani::proof]
/// (auto-generated partition of `input` for condition `|a| *a > 0`)
pub fn partitioned_proof_partition_1() {
    /* OTHER CODE BEFORE THE CALL TO `kani::partition` IF THERE WAS ANY */
    let partition_variable = kani::any();
    kani::assume((|a: &i32| *a > 0)(&partition_variable));
    (|input: i32| target_fn(input))(partition_variable);
}
```

2. an additional proof harness to check that the partitions fully cover the domain of the variable

This would be along the lines of:
```rust
pub fn partitioned_proof_coverage() {
    /* OTHER CODE BEFORE THE CALL TO `kani::partition` IF THERE WAS ANY */
    let partition_variable = kani::any();

    // checks that any possible value of partition_variable will fall into at least one partition.
    let is_in_partitions = [|a: &i32| *a > 0, |a: &i32| *a < 0].into_iter().any(|cond| cond(&partition_variable));
    kani::assert(is_in_partitions, "partition conditions for partitioned_proof do not cover all possible values of a i32.");
}
```

### Corner cases
Although the fact that the partition conditions are function pointers prevents them from capturing any local variables, their value can still be affected by non-deterministic values. 

```rust
#[kani::proof]
pub fn strange_partitioned_proof() {
    let f: fn(&i32) -> bool = if kani::any::<i32>() > 10 {
        |a: &i32| *a < 0
    } else {
        |a: &i32| *a < -2
    };

    kani::partition([f, |a: &i32| *a < 0], ...);
}
```

The behavior of a partition in this case seems ill-defined, so any call to `kani::partition` with conditions that are not constant closures would be disallowed.

## Rationale and alternatives

### 1. Allowing overlapping partition conditions
As a corollary to checking that partitions span the space of all possible inputs, I had initially considered giving a warning if partitions are overlapping as overlap could indicate ill-defined bounds.
TODO: better example...

## Open questions

1. What's the best way to ensure that type errors in the `kani::partitioned_proof` macro are clear, understandable and actionable for users?
2. Can the function names for each partition harness be generated more prettily while still remainging unique? (the current hashes are often opaque for users)
3. Are there any additional correctness issues introduced by this approach?
4. Should verification fully panic if a partitioned proof is lacking full coverage or just have that one assertion fail verification?
5. What's the best way to provide support for partitioning `BoundedArbitrary` types?
Potentially through a separate function `partition_bounded` which would be generic over `T: BoundedArbitrary` and a `const N: usize` which would be used for the call to `kani::bounded_any()`.
6. How should users be able to control our coverage checks? Does it make sense to provide another function that makes the user specify a smaller domain that we can still prove coverage for (or skips those checks entirely)?

[^unstable_feature]: This unique ident should be used to enable features proposed in the RFC using `-Z <ident>` until the feature has been stabilized.