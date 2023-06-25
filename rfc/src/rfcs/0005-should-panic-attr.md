- **Feature Name:** The `kani::should_panic` attribute (`should-panic-attr`)
- **Feature Request Issue:** <https://github.com/model-checking/kani/issues/600>
- **RFC PR:** <https://github.com/model-checking/kani/pull/2272>
- **Status:** Unstable
- **Version:** 0
- **Proof-of-concept:** <https://github.com/model-checking/kani/pull/2315>

-------------------

## Summary

Users may want to express that a verification harness should panic.
This RFC proposes a new harness attribute `#[kani::should_panic]` that informs Kani about this expectation.

## User Impact

Users may want to express that a verification harness should panic.
In general, a user adding such a harness wants to demonstrate that the verification fails because a panic is reachable from the harness.

Let's refer to this concept as *negative verification*,
so the relation with [negative testing](https://en.wikipedia.org/wiki/Negative_testing) becomes clearer.
Negative testing can be exercised in Rust unit tests using [the `#[should_panic]` attribute](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics).
If the `#[should_panic]` attribute is added to a test, `cargo test` will check that the execution of the test results in a panic.
This capability doesn't exist in Kani at the moment, but it would be useful for the same reasons
(e.g., to show that invalid inputs result in verification failures, or increase the overall verification coverage).

We propose an attribute that allows users to exercise negative verification in Kani.

We also acknowledge that, in other cases, users may want to express more granular expectations for their harnesses.
For example, a user may want to specify that a given check is unreachable from the harness.
An ergonomic mechanism for informing Kani about such expectations is likely to require other improvements in Kani (a comprehensive classification for checks reported by Kani, a language to describe expectations for checks and cover statements, and general output improvements).
Moving forward, we consider that such a mechanism and this proposal solve different problems, so they don't need to be discussed together.
This is further discussed in the [rationale and alternatives](#rationale-and-alternatives) and [future possibilities](#future-possibilities) sections.

## User Experience

The scope of this functionality is **limited to the overall verification result**.
The [rationale section](#rationale-and-alternatives) discusses the granularity of failures, and how this attribute could be extended.

### Single Harness

Let's look at this code:

```rust
struct Device {
    is_init: bool,
}

impl Device {
    fn new() -> Self {
        Device { is_init: false }
    }

    fn init(&mut self) {
        assert!(!self.is_init);
        self.is_init = true;
    }
}

#[kani::proof]
fn cannot_init_device_twice() {
    let mut device = Device::new();
    device.init();
    device.init();
}
```

This is what a negative harness may look like.
The user wants to verify that calling `device.init()` more than once should result in a panic.

> **NOTE**: We could convert this into a Rust unit test and add the `#[should_panic]` attribute to it.
> However, there are a few good reasons to have a verification-specific attribute that does the same:
>  1. To ensure that other unexpected behaviors don't occur (e.g., overflows).
>  2. Because `#[should_panic]` cannot be used if the test harness contains calls to Kani's API.
>  3. To ensure that a panic still occurs after stubbing out code which is expected to panic.

Currently, this example produces a `VERIFICATION:- FAILED` result.
In addition, it will return a non-successful code.

```rust
#[kani::proof]
#[kani::should_panic]
fn cannot_init_device_twice() {
    let mut device = Device::new();
    device.init();
    device.init();
}
```

Since we added `#[kani::should_panic]`, running this example would produce a successful code.

Now, we've considered two ways to represent this result in the verification output.
Note that it's important that we provide the user with this feedback:
 1. **(Expectation)** Was Kani expecting the harness to panic?
 2. **(Outcome)**: What's the actual result that Kani produced after the analysis?
This will avoid a potential scenario where the user doesn't know for sure if the attribute has had an effect when verifying the harness.

Below, we show how we'll represent this result.

#### Recommended Representation: Changes to overall result

The representation must make clear both the expectation and the outcome.
Moreover, the overall result must change according to the verification results (i.e., the failures that were found).

Using the `#[kani::should_panic]` attribute will return one of the following results:
 1. `VERIFICATION:- FAILED (encountered no panics, but at least one was expected)` if there were no failures.
 2. `VERIFICATION:- FAILED (encountered failures other than panics, which were unexpected)` if there were failures but not all them had `prop.property_class() == "assertion"`.
 3. `VERIFICATION:- SUCCESSFUL (encountered one or more panics as expected)` otherwise.

Note that the criteria to achieve a `SUCCESSFUL` result depends on all failures having the property class `"assertion"`.
If they don't, then the failed properties may contain UB, so we return a `FAILED` result instead.

### Multiple Harnesses

When there are multiple harnesses, we'll implement the single-harness changes in addition to the following ones.
Currently, a "Summary" section appears[^footnote-summary] after reporting the results for each harness:
```
Verification failed for - harness3
Verification failed for - harness2
Verification failed for - harness1
Complete - 0 successfully verified harnesses, 3 failures, 3 total.
```

Harnesses marked with `#[kani::should_panic]` won't show unless the expected result was different from the actual result.
The summary will consider harnesses that match their expectation as "successfully verified harnesses".

Therefore, if we added `#[kani::should_panic]` to all harnesses in the previous example, we'd see this output:

```
Complete - 3 successfully verified harnesses, 0 failures, 3 total.
```

### Multiple panics

In a verification context, an execution can branch into multiple executions that depend on a condition.
This may result in a situation where different panics are reachable, as in this example:

```rust
#[kani::proof]
#[kani::should_panic]
fn branch_panics() {
    let b: bool = kani::any();

    do_something();

    if b {
        call_panic_1(); // leads to a panic-related failure
    } else {
        call_panic_2(); // leads to a different panic-related failure
    }
}
```

Note that we could safeguard against these situations by checking that only one panic-related failure is reachable.
However, users have expressed that a *coarse* version (i.e., checking that at least one panic can be reached) is preferred.
Users also anticipate that `#[kani::should_panic]` will be used to exercise [smoke testing](https://en.wikipedia.org/wiki/Smoke_testing_(software)) in many cases.
Additionally, restricting `#[kani::should_panic]` to the verification of single panic-related failures could be confusing for users and reduce its overall usefulness.

### Availability

This feature **will only be available as an attribute**.
That means this feature won't be available as a CLI option (i.e., `--should-panic`).
There are good reasons to avoid the CLI option:
 - It'd make the design and implementation unnecessarily complex.
 - It'd only be useful when combined with `--harness` to filter negative harnesses.
 - We could have trouble extending its functionality (see [Future possibilities](#future-possibilities) for more details).

### Pedagogy

The `#[kani::should_panic]` attribute will become one of the most basic attributes in Kani.
As such, it'll be mentioned in the tutorial and added to the dedicated section planned in [#2208](https://github.com/model-checking/kani/issues/2208).

In general, **we'll also advise against negative verification** when a harness can be written both as a regular (positive) harness and a negative one.
The feature, as it's presented in this proposal, won't allow checking that the panic failure is due to the panic we expected. 
So there could be cases where the panic changes, but it goes unnoticed while running Kani.
Because of that, it'll preferred that users write positive harnesses instead.

## Detailed Design

At a high level, we expect modifications in the following components:
 - `kani-compiler`: Changes required to (1) process the new attribute, and (2) extend `HarnessMetadata` with a `should_panic: bool` field.
 - `kani-driver`: Changes required to (1) edit information about harnesses printed by `kani-driver`,  (2) edit verification output when post-processing CBMC verification results, and (3) return the appropriate exit status after post-processing CBMC verification results.

We don't expect these changes to require new dependencies.
Besides, we don't expect these changes to be updated unless we decide to extend the attribute with further fields (see [Future possibilities](#future-possibilities) for more details).

## Rationale and alternatives

This proposal would enable users to exercise negative verification with a relatively simple mechanism.
Not adding such a mechanism could impact Kani's usability by limiting the harnesses that users can write.

### Alternative #1: Generic failures

This proposal **doesn't consider generic failures but only panics**.
In principle, it's not clear that a mechanism for generic failures would be useful.
Such a mechanism would allow users to expect UB in their harness, but there isn't a clear motivation for doing that.

### Alternative #2: Name

We have considered two alternatives for the "expectation" part of the attribute's name: `should` and `expect`.
We avoid `expect` altogether for two reasons:
 - We may consider adding the `expected` argument to `#[kani::should_panic]`.
 - We may consider a more granular approach to indicate expectations regarding individual checks and cover statements in the future. One possible name for the attribute is `#[kani::expect]`.
 - We heavily use this word for testing in Kani: there is an `expected` mode, which works with `*.expected` files. Other modes also use such files.

### Alternative #3: The `expected` argument

We could consider an `expected` argument, similar to [the `#[should_panic]` attribute](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics).
To be clear, the `#[should_panic]` attribute may receive an argument `expected` which allows users to specify the expected panic string:

```rust
    #[test]
    #[should_panic(expected = "Divide result is zero")]
    fn test_specific_panic() {
        divide_non_zero_result(1, 10);
    }
```

In principle, we anticipate that we'll extend this proposal to include the `expected` argument at some point.
The implementation could compare the `expected` string against the panic string.

At present, the only technical limitation is that panic strings printed in Kani aren't formatted.
One option is to use substrings to compare.
However, the long-term solution is to use concrete playback to *replay* the panic and match against the expected panic string.
By doing this, we would achieve feature parity with Rust's `#[should_panic]`.

### Alternative #4: Granularity

As mentioned earlier, users may want to express more granular expectations for their harnesses.

There could be problems with this proposal if we attempt to do both:
 * What if users don't want to only check for failures (e.g., reachability)?
 * In the previous case, would they expect the overall verification to fail or not?
 * How do we want these expectations to be declared?

We don't have sufficient data about the use-case considered in this alternative.
This proposal can also contribute to collect this data: once users can expect panics, they may want to expect other things.

### Alternative #5: Kani API

This functionality could be part of the Kani API instead of being an attribute.
For example, some contributors proposed a function that takes a predicate closure to filter executions and check that they result in a panic.

However, such a function couldn't be used in external code, limiting its usability to the user's code.

## Open questions

Once the feature is available, it'd be good to gather user feedback to answer these questions:
 - Do we need a mechanism to express more granular expectations?
 - If we need the mechanism in (2), do we really want to collapse them into one feature?

### Resolved questions

 - *What is the best representation to use for this feature?* A representation that changes the overall result seems to be preferred, according to feedback we received during a discussion.
 - *Do we want to extend `#[kani::should_panic]` with an `expected` field?* Yes, but not in this version.
 - *Do we want to allow multiple panic-related failures with `#[kani::should_panic]`?* Yes (this is now discussed in [User Experience](#user-experience)).

## Future possibilities

 - The attribute could be an argument to `kani::proof` (`#[kani::proof(should_panic)]` reads very well).
 - Add an `expected` argument to `#[kani::should_panic]`, and *replay* the harness with concrete playback to get the actual panic string.

[^footnote-representation]: Double negation may not be the best representation, but it's at least accurate with respect to the original result.

[^footnote-summary]: This summary is printed in both the default and terse outputs.
