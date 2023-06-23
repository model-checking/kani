# FAQs

This section collects frequently asked questions about Kani.
Please consider [opening an issue](https://github.com/model-checking/kani/issues/new/choose) if you have a question that would like to see here.

## Questions

<details>
<summary>Kani doesn't fail after <code>kani::assume(false)</code>. Why?</summary>
</br>

`kani::assume(false)` (or `kani::assume(cond)` where `cond` is a condition that results in `false` in the context of the program), won't cause errors in Kani.
Instead, such an assumption has the effect of blocking all the symbolic execution paths from the assumption.
Therefore, all checks after the assumption should appear as [`UNREACHABLE`](#../../verification-results.md).
That's the expected behavior for `kani::assume(false)` in Kani.

If you didn't expect certain checks in a harness to be `UNREACHABLE`, we recommend using the [`kani::cover` macro](#../../verification-results.md#cover-property-results) to determine what conditions are possible in case you've over-constrained the harness.
</details>

<details>
<summary>I implemented the <code>kani::Arbitrary</code> trait for a type that's not from my crate, and got the error
<code>only traits defined in the current crate can be implemented for types defined outside of the crate</code>.
What does this mean? What can I do?</summary>
</br>

This error is due to a violation of Rust's orphan rules for trait implementations, which are explained [here](https://doc.rust-lang.org/error_codes/E0117.html).
In that case, you'll need to write a function that builds an object from non-deterministic variables.
Inside this function you would simply return an arbitrary value by generating arbitrary values for its components.

For example, let's assume the type you're working with is this enum:

```rust
#[derive(Copy, Clone)]
pub enum Rating {
    One,
    Two,
    Three,
}
```

Then, you can match on a non-deterministic integer (supplied by `kani::any`) to return non-deterministic `Rating` variants:

```rust
    pub fn any_rating() -> Rating {
        match kani::any() {
            0 => Rating::One,
            1 => Rating::Two,
            _ => Rating::Three,
        }
    }
```

More details about this option, which also useful in other cases, can be found [here](https://model-checking.github.io/kani/tutorial-nondeterministic-variables.html#custom-nondeterministic-types).

If the type comes from `std` (Rust's standard library), you can [open a request](https://github.com/model-checking/kani/issues/new?assignees=&labels=%5BC%5D+Feature+%2F+Enhancement&template=feature_request.md&title=) for adding `Arbitrary` implementations to the Kani library.
Otherwise, there are more involved options to consider:
 1. Importing a copy of the external crate that defines the type, then implement `Arbitrary` there.
 2. Contributing the `Arbitrary` implementation to the external crate that defines the type.
</details>
