# FAQs

This section collects frequently asked questions about Kani.
Please consider [opening an issue](https://github.com/model-checking/kani/issues/new/choose) if you have a question that would like to see here.

## Questions

<details>
<summary>Kani doesn't fail after `kani::assume(false)`. Why?</summary>

`kani::assume(false)` (or `kani::assume(cond)` where `cond` is condition that results in `false` in the context of the program), won't cause errors in Kani.
Instead, such an assumption has the effect of blocking all the symbolic execution paths from the assumption.
Therefore, all checks after the assumption should appear as [`UNREACHABLE`](#../../verification-results.md).
That's the expected behavior for `kani::assume(false)` in Kani.

If you didn't expect certain checks in a harness to be `UNREACHABLE`, we recommend using the [`kani::cover` macro](#../../verification-results.md#cover-property-results) to determine what conditions are possible in case you've over-constrained the harness.
</details>

<details>
<summary>I implemented the `kani::Arbitrary` trait for a type that's not from my crate, and got the error
`only traits defined in the current crate can be implemented for types defined outside of the crate`.
What does this mean? What can I do?</summary>

This error is due to a violation of Rust's orphan rules for trait implementations, which are explained [here](https://doc.rust-lang.org/error_codes/E0117.html).
In that case, you'll need to follow the third approach mentioned [here](https://model-checking.github.io/kani/tutorial-nondeterministic-variables.html#custom-nondeterministic-types) to implement `Arbitrary` for a foreign custom type.

If the type comes from `std` (Rust's standard library), you can [open a request](https://github.com/model-checking/kani/issues/new?assignees=&labels=%5BC%5D+Feature+%2F+Enhancement&template=feature_request.md&title=) for adding `Arbitrary` implementations to the Kani library.
Otherwise, there are more involved options to consider:
 1. Importing a copy of the external crate that defines the type, then implement `Arbitrary` there.
 2. Contributing the `Arbitrary` implementation to the external crate that defines the type.
</details>


**Question:** I implemented the `kani::Arbitrary` trait for a type that's not from my crate, and got the error
`only traits defined in the current crate can be implemented for types defined outside of the crate`.
What does this mean? What can I do? [Answer](#arbitrary-implementations-for-foreign-types)
