- **Feature Name:** Harness Partition (`harness-partition`)
- **Feature Request Issue:** [#3006](https://github.com/model-checking/kani/issues/3006)
- **RFC PR:** *Link to original PR*
- **Status:** *One of the following: [Under Review | Unstable | Stable | Cancelled]*
- **Version:** 0
- **Proof-of-concept:** *Optional field. If you have implemented a proof of concept, add a link here*

-------------------

## Summary

It can often be useful to subdivide an expensive proof harness so that different parts of the input space are verified separately. Adding the built-in ability to partition a proof harness into different pieces (that each make differing assumptions about their inputs) could reduce the cost of expensive proofs, while allowing us to automatically check that the partitions cover the entire input space and, thus, will not affect soundness.

## User Impact

Imagine that you have a function to verify like the following (based on the example from [#3006](https://github.com/model-checking/kani/issues/3006)). Since there are two tricky to analyze functions, but only one will ever be called on a given input, you might want to verify all inputs that will take the first branch separately from those that will take the second. This way, each solve would be smaller in isolation, and you'd be able to take advantage of CBMC's internal parallelism by running both proofs at once.

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

The best way to currently do this would be to manually partition out these paths into two proof harnesses.

```rust
#[kani::proof]
pub fn first_branch_harness() {
    let input = kani::any_where(|i: &i32| *i > 0i32);
    assert!(target_fn(input) > 0)
}

#[kani::proof]
pub fn second_branch_harness() {
    let input = kani::any_where(|i: &i32| *i < 0i32); // This should've been i <= 0
    assert!(target_fn(input) > 0)
}
```

However, this can affect soundess, as there's no guarantee that your partitions will fully span the space of possible inputs. The only way to determine that a set of proofs like the one above are incorrect (as it forgets to account for when `i == 0`) is by manual inspection, which gets infeasible for proofs with complex types or partition rules.

It would be helpful if Kani provided this as a built-in feature that could reason about given partition conditions to provide soundess guarantees.

## User Experience

The current thought is for users to interact with this using a new `#[kani::partitioned_proof()]` attribute where they provide the conditions by which to partition the input space of their proof.

For example, the above would become

```rust
#[kani::partitioned_proof(partitions = [|a: &i32| { *a > 0 }, |a: &i32| { *a < 0 }])]
pub fn partitioned_harness(input: i32) {
    assert!(target_fn(input) > 0)
}
```

And Kani would automatically handle checking the partition conditions for soundess and generating the partitioned proofs.

This should be a description on how users will interact with the feature.
Users should be able to read this section and understand how to use the feature.
**Do not include implementation details in this section, neither discuss the rationale behind the chosen UX.**

Please include:
  - High level user flow description.
  - Any new major functions or attributes that will be added to Kani library.
  - New command line options or subcommands (no need to mention the unstable flag).
  - List failure scenarios and how are they presented (e.g., compilation errors, verification failures, and possible failed user iterations).
  - Substantial changes to existing functionality or Kani output.

If the RFC is related to architectural changes and there are no visible changes to UX, please state so.
No further explanation is needed.

## Software Design

A prototype of this design has been implemented locally [here](https://github.com/AlexanderPortland/kani/tree/harness-partitioning).

It works by 

**We recommend that you leave the Software Design section empty for the first version of your RFC**.

This is the beginning of the technical portion of the RFC.
From now on, your main audience is Kani developers, so it's OK to assume readers know Kani architecture.

Please provide a high level description your design.

- What are the main components that will be modified? (E.g.: changes to `kani-compiler`, `kani-driver`, metadata, proc-macros, installation...)
- Will there be changes to the components interface?
- Any changes to how these components communicate?
- What corner cases do you anticipate?

## Rationale and alternatives

The ``

This is the section where you discuss the decisions you made.

- What are the pros and cons of the UX? What would be the alternatives?
- What is the impact of not doing this?
- Any pros / cons on how you designed this?

## Open questions

1. What's the best way to ensure that type errors in the `kani::partitioned_proof` macro are clear and fixable for users?
2. 

List of open questions + an optional link to an issue that captures the work required to address the open question.
Capture the details of each open question in their respective issue, not here.

Example:
- Is there any use case that isn't handled yet?
- Is there any part of the UX that still needs some improvement?

Make sure all open questions are addressed before stabilization.

## Out of scope / Future Improvements

*Optional Section*: List of extensions and possible improvements that you predict for this feature that is out of
the scope of this RFC.

Feel free to add as many items as you want, but please refrain from adding too much detail.
If you want to capture your thoughts or start a discussion, please create a feature request.
You are welcome to add a link to the new issue here.

[^unstable_feature]: This unique ident should be used to enable features proposed in the RFC using `-Z <ident>` until the feature has been stabilized.
