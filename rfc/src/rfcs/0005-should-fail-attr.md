- **Feature Name:** *Fill me with pretty name and a unique ident. E.g: New Feature (`new_feature`)*
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/600>
- **RFC PR:** *Link to original PR*
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** *Optional field. If you have implemented a proof of concept, add a link here*

## Summary

Users may want to express that a verification harness should fail.
This RFC proposes a new harness attribute `#[kani::should_fail]` that informs Kani about this expectation.

## User Impact

Users may want to express that a verification harness should fail for multiple reasons.
In general, users writing such harnesses want to demonstrate that their verification results in a failure.
Let's refer to this concept as *negative verification*, inspired by the term [negative testing](https://en.wikipedia.org/wiki/Negative_testing).
We may consider the following reasons for users to write negative verification harnesses:
 * To improve the coverage achieved with verification.
 * To demonstrate that invalid input ends up produing errors.

In fact, the closest example of this are some of our tests.
Currently, we use test annotations specific to `compiletest` to indicate that a failure is expected.
The proposed annotation would provide a way in Kani to express that a verification harness should fail,
allowing users to write verification harness that are expected to fail.


---
Why are we doing this? How will this benefit the final user?

 - If this is an API change, how will that impact current users?
 - For deprecation or breaking changes, how will the transition look like?
 - If this RFC is related to change in the architecture without major user impact, think about the long term
impact for user. I.e.: what future work will this enable.

## User Experience

The scope of this functionality is limited to the overall verification result.
The [rationale section](#rationale-and-alternatives) discusses the granularity of failures, and how this attribute could be extended.

### Single Harness

Let's take one of the simplest examples from our regression:

```rust
#[kani::proof]
fn simple_add_overflows() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    a + b;
}
```

Currently, this example produces a `VERIFICATION:- FAILED` result.[^footnote-compiletest]

```rust
#[kani::proof]
#[kani::should_fail]
fn simple_add_overflows() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    a + b;
}
```

Since we added `#[kani::should_fail]`, running this example would produce a successful verification code.

Now, we've considered two ways to represent this result in the verification output.
Note that it's important that we provide the user with this feedback:
 1. **(Expectation)** Kani was expecting the harness to fail.
 2. **(Transparency)**: The actual verification result that Kani produced after the analysis.
This will avoid a potential scenario where the user doesn't know for sure if the attribute has had an effect when running the verification harness.

As mentioned, we've considered two ways to represent this result.

#### Representation #1 (Recommended): No changes to overall result

```rust
VERIFICATION:- FAILED (should have FAILED)
```

#### Representation #2: Changes to overall result

```rust
VERIFICATION:- SUCCESSFUL (should have FAILED and it FAILED)
```

### Multiple Harnesses

When there are multiple harness, we'll implement the single-harness changes in addition to these ones.
Currently, a "Summary" section appears after reporting the results for each harness:
```
Verification failed for - harness3
Verification failed for - harness2
Verification failed for - harness1
Complete - 0 successfully verified harnesses, 3 failures, 3 total.
```

Harnesses marked with `#[kani::should_fail]` won't show unless the expected result was different from the actual result.
The summary will consider harness that match the expectation as "successfully verified harnesses".

Therefore, if we added `#[kani::should_fail]` to all harnesses in the previous example, we'd see this output:
```
Complete - 3 successfully verified harnesses, 0 failures, 3 total.
```

### Availability

This feature will only be available as an attribute.
That means this feature won't be available as a CLI option (i.e., `--should-fail`).
There are good reasons to avoid the CLI option:
 - It'd make the design and implementation unnecessarily complex.
 - It'd only be useful when combined with `--harness` to filter negative harnesses.
 - We could have trouble extending its functionality (see [Future possibilities](#future-possibilities) for more details).

### Pedagogy

The `#[kani::should_fail]` attribute will become on the most basic attributes in Kani.
As such, it'll be mentioned in the tutorial and added to the dedicated section planned in [#2208](https://github.com/model-checking/kani/issues/2208).

## Detailed Design

At a high level, we expect modifications in the following components:
 - `kani-compiler`: Changes required to (1) process the new attribute, and (2) extend `HarnessMetadata` with a `should_fail: bool` field.
 - `kani-driver`: Changes required to (1) edit information about harnesses printed by `kani-driver`,  (2) edit verification output when post-processing CBMC verification results, and (3) return the appropriate exit status after post-processing CBMC verification results.

We don't expect these changes to require new dependencies.
Besides, we don't expect these changes to be updated unless we decide to extend the attribute with further fields (see [Future possibilities](#future-possibilities) for more details).

## Rationale and alternatives

This proposal would enable users to exercise negative verification with a relatively
Moreover, we could use it in our own regression and remove special-cased code to handle this case in `compiletest`.
This is a relatively cheap feature to implement if we compare it take into account the expressivenes it provides.

### Alternative #1: Name

Our first choice is made in the name of the attribute.
It should be noted that Rust has a similarly named attribute for unit tests: [the `#[should_panic]` attribute](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics).
At first, one may think that `#[kani::should_fail]` and `#[should_panic]` have the same semantics.
However, if we keep thinking about this, we'll realize that:
 1. A panic in Rust should always result in a failure in Kani, but
 2. A failure in Kani doesn't always result in a panic in Rust.

Our running example shows an instance of (2): an overflow error doesn't result in a panic in Rust.
So it's better to avoid `panic` in the name.

We have also considered two alternatives for the expectation: `should` and `expect`.
To be honest, we've avoided `expect` altogether for two reasons:
 - We may consider a more granular approach to indicate expectations regarding individual checks and cover statements in the future. The tentative name for the attribute is `#[kani::expect]`.
 - We heavily use this word for testing in Kani.

### Alternative #2: Granularity



- What are the pros and cons of this design?
- What is the impact of not doing this?
- What other designs have you considered? Why didn't you choose them?

## Open questions

- Is there any part of the design that you expect to resolve through the RFC process?
- What kind of user feedback do you expect to gather before stabilization? How will this impact your design?

## Future possibilities

What are natural extensions and possible improvements that you predict for this feature that is out of the
scope of this RFC? Feel free to brainstorm here.

 * More granular checks.
 * Attribute auto-generation from `kani::proof`

[footnote-compiletest]: `compiletest` knows that the test should fail because it parses the `// kani-verify-fail` comment at the top.
