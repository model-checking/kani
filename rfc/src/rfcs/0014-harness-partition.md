- **Feature Name:** Harness Partition (`harness-partition`)
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

The current thought is for users to interact with this using a new `#[kani::partitioned_proof()]` attribute where they provide the conditions by which to partition the input space of their proof.
Each condition must be of type `fn(&T) -> bool` (where `T` is the input type to the proof body that implements `kani::Arbitrary`) and is used to filter which values are part of that partition.

For example, the above would become

```rust
#[kani::partitioned_proof(|a: &i32| { *a > 0 }, |a: &i32| { *a < 0 })]
pub fn partitioned_harness(input: i32) {
    assert!(target_fn(input) > 0)
}
```

And Kani would automatically handle checking the partition conditions for soundness and generating the partitioned proofs.
Overlaps would be allowed as they don't affect soundness and may be useful in certain cases (see [below](#2-checking-for-overlapping-partitions) for more discussion).

Generally, use of this feature would look like the following (where `T` is an arbitrary input type).

```rust
#[kani::partitioned_proof(|a: &T| { #condition_1# }, |a: &T| { #condition_2# }, ..., |a: &T| { #condition_n# })]
pub fn harness(input: T) {
    /* your interesting assertions here */
}
```

## Software Design

A prototype of this design has been implemented locally [here](https://github.com/AlexanderPortland/kani/tree/harness-partitioning).

It introduces the `#[kani::partitioned_proof()]` attribute as variant of `#[kani::proof]` that takes in a set of closures representing the partition bounds.
With those bounds, it keeps the main harness function the same, but does not annotate it with the `#kani_attributes` that would typically mark it as an entrypoint for Kani's verification.
Instead, for each partition, it will generate a new function definition for that portion of the proof.
These functions are just wrappers that constrain an arbitrary value to be within the partition and then call the real harness that will contain the meat of the proof.
Their names are derived from a hash of their condition closure for uniqueness (but see [below](#open-questions) for discussion on how to improve this).
So, for the running example above, the call to `kani::partitioned_proof` generates:

```rust
#[kani::proof]
//                         (hash of |a| *a > 0)
pub fn partitioned_harness_7199877664941740246() {
    let t = kani::any_where(|a: &i32| { *a > 0 });
    partitioned_harness(t)
}

#[kani::proof]
//                         (hash of |a| *a < 0)
pub fn partitioned_harness_2423413845937420568() {
    let t = kani::any_where(|a: &i32| { *a < 0 });
    partitioned_harness(t)
}
```

It will then also inject a new proof that has an assert to ensure that all possible values fall into one of the given partitions.
In this case, it will fail to be verified (as `t` can be 0), telling the user that their partitions are not complete and could affect proof soundness.


```rust
#[kani::proof]
pub fn partitioned_harness_missing_full_coverage() {
    let t: i32 = kani::any();
    let partitions: [fn(&i32) -> bool; 2] = [|a: &i32| { *a > 0 }, |a: &i32| { *a < 0 }];
    let partitions_have_full_coverage: bool = partitions.into_iter().any(|condition| condition(&t));
    assert!(partitions_have_full_coverage)
}
```

### Corner cases
1. This current implementation could run into issues if the main proof harness & partition condition closures both don't specify concrete types (e.g. by using generics), as the compiler may not be able to determine what `T` to use `kani::any()` on.
This should be fixed by explicitly enforcing the current implicit assumption that the input type is a concrete type.

2. This implementation doesn't play nicely if your proof is intentionally only covers a subspace of inputs (e.g. if you're trying to partition a proof that initially used `kani::any_where(...)`).
(see the open questions [below](#open-questions) for thoughts on how to fix this)

## Rationale and alternatives

### 1. Alternative APIs
The `kani::partition([i > 0, i < 0], || target_fn(input))` syntax suggested ([here](https://github.com/model-checking/kani/issues/3006#issue-2123964835)) in the initial issue is clear, but runs into some implementation issues irregardless of whether it's a function call or macro.
The goal of this feature is to generate multiple proof harnesses from the single call to `partition`, and this is very difficult to do that from a macro that's already inside a function (as your codegen is limited to within that function's scope).

The design described above is simpler as it is an attribute macro and, thus, has access to the program's global scope, allowing it to keep the proof body the same and simply generate additional function definition wrappers for each partitioned proof.

This was also initially implemented as an addition to the existing `#[kani::proof]` attribute, where users would specify partition conditions with `kani::proof(partitions = [|a| a % 2 == 9, ...])`.
However, since these kinds of proofs require that the annotated function take an input, while `#[kani::proof]` functions typically don't, it seemed more consistent to introduce a separate attribute macro.

### 2. Checking for overlapping partitions
As a corellary to checking that partitions span the space of all possible inputs, I had initially considered giving a warning if partitions are overlapping as overlap could indicate ill-defined bounds.
However, I decided against this, as overlapping partitions may be useful in certain cases where the user wishes to purposefully oversimplify their partition conditions in a way that will cause overlap but make them easier to express.

## Open questions

1. What's the best way to ensure that type errors in the `kani::partitioned_proof` macro are clear, understandable and actionable for users?
2. Can function names be generated more prettily while still remainging unique? (the current hashes are often opaque for users)
3. Are there any additional correctness issues introduced by this approach?
4. Is there a way to simplify the generated `..._full_coverage()` proof harness? The iteration seems to be instrumented by our code which adds additional checks.
5. Is there a way to promote the fact that verification failed on the `..._full_coverage()` proof harness as an error that could affect soundness? If Kani's users tend to look into any failed checks, this may not be needed, but it may be possible for a single failure to get lost in big projects.
6. Would it be desirable to add an optional argument that specifies which subset of inputs you're trying to verify with the proof as a whole (if not provided just the whole input space)?

[^unstable_feature]: This unique ident should be used to enable features proposed in the RFC using `-Z <ident>` until the feature has been stabilized.
