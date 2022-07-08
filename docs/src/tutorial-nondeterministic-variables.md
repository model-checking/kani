# Nondeterministic variables

Kani is able to reason about programs and their execution paths by allowing users to assign nondeterministic (also called symbolic) values to certain variables using `kani::any()`.
Kani is a "bit-precise" model checker, which means that Kani considers all the possible bit-value combinations that could be assigned to the variable's memory contents.

As a Rust developer, this sounds a lot like the `mem::transmute` operation, which is highly `unsafe`.
And that's correct.
But `kani::any()` is only implemented for a few types: those where we are able to safely enforce the type's invariants.

In other words, `kani::any()` should not produce values that are invalid for the type.

## Safe nondeterministic variables

Let's say you're developing an inventory management tool, and you would like to start verifying properties about your API.
Here is a simple example (available [here](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/arbitrary-variables/src/inventory.rs)):

```rust
{{#include tutorial/arbitrary-variables/src/inventory.rs:inventory_lib}}
```

Let's write a fairly simple proof harness, one that just ensures we successfully `get` the value we inserted with `update`:

```rust
{{#include tutorial/arbitrary-variables/src/inventory.rs:safe_update}}
```

We use `kani::any()` twice here:

1. `id` has type `ProductId` which was actually just a `u32`, and so any value is fine.
2. `quantity`, however, has type `NonZeroU32`.
In Rust, it would be undefined behavior to have a value of `0` for this type.

We included an extra assertion that the value returned by `kani::any()` here was actually non-zero.
If we run this, you'll notice that verfication succeeds.

```bash
cargo kani --harness safe_update
```

`kani::any()` is safe Rust, and so Kani only implements it for types where type invariants are enforced.
For `NonZeroU32`, this means we never return a `0` value.
The assertion we wrote in this harness was just an extra check we added to demonstrate this fact.

## Other kinds of nondeterministic variables

There's nothing special about `kani::any()` except that Kani ships with it only implemented for types where we can guarantee safety.
Notably, however, there is no implementation for `Vec<T>`, for example.

The trouble with a nondeterministic vector is that you usually need to _bound_ the size of the vector, as we saw in the last chapter.
There are no arguments provided to `kani::any()` to indicate an upper bound.

Likewise, `kani::any()` is implemented for `[T; N]` (if implemeneted for `T`).
But, it is _not_ implemented for `&[T]`.
Again, there's no upper bound information we can make use of.

This does not mean you cannot have a nondeterministic vector.
You just have to construct it.

<!-- TODO: Once we're sure we have mature, maintainable APIs, let's document any_vec, any_slice (any_string?) here -->

<!-- TODO: Current `Invariant` API is basically undefined behavior by default. Not documenting here for now.
     There's currently no advantage to implementing `Invariant` versus just writing a function `any_my_type() -> T` anyway.
  -->

