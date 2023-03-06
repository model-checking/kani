- **Feature Name:** The `kani::should_fail` attribute (`should-fail-attr`)*
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/600>
- **RFC PR:** *Link to original PR*
- **Status:** Under Review
- **Version:** 0
- **Proof-of-concept:** *Optional field. If you have implemented a proof of concept, add a link here*

## Summary

Users may want to express that a verification harness should fail.
This RFC proposes a new harness attribute `#[kani::should_fail]` that informs Kani about this expectation.

## User Impact

Users may want to express that a verification harness should fail.
In general, users writing such harnesses want to demonstrate that verification results include at least a failure.
Let's refer to this concept as *negative verification*, inspired by the term [negative testing](https://en.wikipedia.org/wiki/Negative_testing).
Negative verification harnesses could be useful to show that invalid inputs result in verification failures,
or increase the overall verification coverage.

In fact, some of the tests in the Kani regression are written as negative harnesses.
Currently, we [use comments](https://model-checking.github.io/kani/regression-testing.html#testing-stages) specific to `compiletest` to indicate that a verification failure is expected.
The proposed attribute would provide a way in Kani to express that a verification harness should fail,
allowing users to write negative verification harnesses.

We also acknowledge that, in other cases, users may want to express more granular expectations for their harnesses.
For example, a user may want to specify that a given check results in a failure.
An ergonomic mechanism for informing Kani about such expectations is likely to require other improvements in Kani (a comprehensive classification for checks reported by Kani, a language to describe expectations for checks and cover statements, and general output improvements).
We consider that the mechanism we just mentioned and the one discussed in this proposal solve different problems, so they don't need to be discussed together.
This is further discussed in the [rationale and alternatives](#rationale-and-alternatives) and [future possibilities](#future-possibilities) sections.

## User Experience

The scope of this functionality is limited to the overall verification result.
The [rationale section](#rationale-and-alternatives) discusses the granularity of failures, and how this attribute could be extended.

### Single Harness

Let's take an example from the Kani regression:

```rust
#[kani::proof]
fn simple_add_overflows() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    a + b;
}
```

Currently, this example produces a `VERIFICATION:- FAILED` result.[^footnote-compiletest]
In addition, it will return a non-successful code.

```rust
#[kani::proof]
#[kani::should_fail]
fn simple_add_overflows() {
    let a: u32 = kani::any();
    let b: u32 = kani::any();
    a + b;
}
```

Since we added `#[kani::should_fail]`, running this example would produce a successful code.

Now, we've considered two ways to represent this result in the verification output.
Note that it's important that we provide the user with this feedback:
 1. **(Expectation)** Kani was expecting the harness to fail.
 2. **(Outcome)**: The actual verification result that Kani produced after the analysis.
This will avoid a potential scenario where the user doesn't know for sure if the attribute has had an effect when running the verification harness.

As mentioned, we've considered two ways to represent this result.

#### Representation #1 (Recommended): No changes to overall result

```rust
VERIFICATION:- FAILED (expected FAILED)
```

We recommend this representation so the user receives clear information about both the outcome and the expectation.

#### Representation #2: Changes to overall result

```rust
VERIFICATION:- SUCCESSFUL (expected FAILED and it FAILED)
```

### Multiple Harnesses

When there are multiple harnesses, we'll implement the single-harness changes in addition to the following ones.
Currently, a "Summary" section appears after reporting the results for each harness:
```
Verification failed for - harness3
Verification failed for - harness2
Verification failed for - harness1
Complete - 0 successfully verified harnesses, 3 failures, 3 total.
```

Harnesses marked with `#[kani::should_fail]` won't show unless the expected result was different from the actual result.
The summary will consider harnesses that match their expectation as "successfully verified harnesses".

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

The `#[kani::should_fail]` attribute will become one of the most basic attributes in Kani.
As such, it'll be mentioned in the tutorial and added to the dedicated section planned in [#2208](https://github.com/model-checking/kani/issues/2208).

## Detailed Design

At a high level, we expect modifications in the following components:
 - `kani-compiler`: Changes required to (1) process the new attribute, and (2) extend `HarnessMetadata` with a `should_fail: bool` field.
 - `kani-driver`: Changes required to (1) edit information about harnesses printed by `kani-driver`,  (2) edit verification output when post-processing CBMC verification results, and (3) return the appropriate exit status after post-processing CBMC verification results.

We don't expect these changes to require new dependencies.
Besides, we don't expect these changes to be updated unless we decide to extend the attribute with further fields (see [Future possibilities](#future-possibilities) for more details).

## Rationale and alternatives

This proposal would enable users to exercise negative verification with a relatively simple mechanism.
Moreover, it could be used in the Kani regression and remove the special-cased code to handle negative harnesses in `compiletest`.

Not adding such a mechanism could impact Kani's usability by limiting the harnesses that users can write.

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
To be honest, we avoid `expect` altogether for two reasons:
 - We may consider a more granular approach to indicate expectations regarding individual checks and cover statements in the future. One possible name for the attribute is `#[kani::expect]`.
 - We heavily use this word for testing in Kani: there is an `expected` mode, which works with `*.expected` files. Other modes also use such files.

### Alternative #2: Granularity

As mentioned earlier, users may want to express more granular expectations for their harnesses.
For example, a user may want to specify that a given check results in a failure.

Again, there is a relation between this and the [the `#[should_panic]` attribute](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics) for Rust tests.
More specifically, the `#[should_panic]` attribute may receive an argument `expected` which allows users to specify the expected panic string:

```rust
    #[test]
    #[should_panic(expected = "Divide result is zero")]
    fn test_specific_panic() {
        divide_non_zero_result(1, 10);
    }
```

In principle, it wouldn't be a problem to extend this proposal to include the `expected` argument.
The implementation could compare the `expected` string against property descriptions.

However, there could be some problems later with this approach:
 * What if users want to specify multiple `expected` strings?
 * What if users don't want to only check for failures?
 * In the previous case, would they expect the overall verification to fail or not?

We don't know if these will be requirements in the future.
If they were, then we'd need to come up with language to state expectations about properties (checks or cover statements).
A good option to consider here is adding the single-string `expected` argument (as in `#[should_panic]`) and see how it's used.
It could be the cause of a breaking change in the future, but it would provide us with useful data in the meatime.

## Open questions

The main question I'd like to get resolved are:
 * Do we want to extend `#[kani::should_fail]` with an `expected` field?

Once the feature is available, it'd be good to gather user feedback to answer these questions:
 1. Do we need a mechanism to express more granular expectations or it's (1) enough?
 2. If we need the mechanism in (2), do we really want to collapse them into one feature?

## Future possibilities

 * The attribute could be an argument to `kani::proof` (`#[kani::proof(should_fail)]` reads very well).
 * Other extensions have been discussed earlier.

[footnote-compiletest]: `compiletest` knows that the test should fail because it parses the `// kani-verify-fail` comment at the top.
