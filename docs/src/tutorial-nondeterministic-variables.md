# Nondeterministic variables

Kani is able to reason about programs and their execution paths by allowing users to create nondeterministic (also called symbolic) values using `kani::any()`.
Kani is a "bit-precise" model checker, which means that Kani considers all the possible bit-value combinations _that would be valid_ if assigned to a variable's memory contents.
In other words, `kani::any()` should not produce values that are invalid for the type (which would lead to Rust undefined behavior).

Out of the box, Kani includes `kani::any()` implementations for most primitive and some `std` types.
In this tutorial, we will show how to use `kani::any()` to create symbolic values for other types. 

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
If we run this, you'll notice that verification succeeds.

```bash
cargo kani --harness safe_update
```

`kani::any()` is safe Rust, and so Kani only implements it for types where type invariants are enforced.
For `NonZeroU32`, this means we never return a `0` value.
The assertion we wrote in this harness was just an extra check we added to demonstrate this fact, not an essential part of the proof.

## Custom nondeterministic types

While `kani::any()` is the only method Kani provides to inject non-determinism into a proof harness, Kani only ships with implementations for a few `std` types where we can guarantee safety.
When you need nondeterministic variables of types that don't have a `kani::any()` implementation available, you have the following options:

1. Implement the `kani::Arbitrary` trait for your type, so you and downstream crates can use `kani::any()` with your type. See [the Arbitrary trait documentation](./reference/arbitrary.md) for details.
2. Implement the [`bolero_generator::TypeGenerator` trait](https://docs.rs/bolero-generator/0.8.0/bolero_generator/trait.TypeGenerator.html).
This will enable you and downstream crates to use Kani via [Bolero](https://camshaft.github.io/bolero/).
3. Write a function that builds an object from non-deterministic variables.

We recommend the first approach for most cases.
The first approach is simple and conventional. This option will also enable you to use it with parameterized types, such as `Option<MyType>` and arrays.
Kani includes a derive macro that allows you to automatically derive `kani::Arbitrary` for structures and enumerations as long as all its fields also implement the `kani::Arbitrary` trait.
One downside of this approach today is that the `kani` crate ships with Kani, but it's not yet available on [crates.io](https://crates.io).
So you need to annotate the Arbitrary implementation with a `#[cfg(kani)]` attribute.
For the derive macro, use `#[cfg_attr(kani, derive(kani::Arbitrary))]`.

The second approach is recommended for cases where you would also like to be able to apply fuzzing or property testing.
The benefits of doing so were described in [this blog post](https://model-checking.github.io/kani-verifier-blog/2022/10/27/using-kani-with-the-bolero-property-testing-framework.html).
Like `kani::Arbitrary`, this trait can also be used with a `derive` macro.
One thing to be aware of is that this type allow users to generate arbitrary values that include pointers.
In those cases, **only the values pointed to are arbitrary**, not the pointers themselves.

Finally, the last approach is recommended when you need to pass in parameters, like bounds on the size of the data structure.
(Which we'll discuss more in the next section.)
This approach is also necessary when you need to generate a nondeterministic variable of a type that you're importing from another crate, since Rust doesn't allow you to implement a trait defined in an external crate for a type that you don't own.

Either way, inside this function you would simply return an arbitrary value by generating arbitrary values for its components.
To generate a nondeterministic struct, you would just generate nondeterministic values for each of its fields.
For complex data structures like vectors or other containers, you can start with an empty one and add a (bounded) nondeterministic number of entries.

For example, for a simple enum you can just annotate it with the Arbitrary derive attribute:

```rust
{{#include tutorial/arbitrary-variables/src/rating.rs:rating_enum}}
```

But if the same enum is defined in an external crate, you can use a simple trick:

```rust
{{#include tutorial/arbitrary-variables/src/rating.rs:rating_arbitrary}}
```

All we're doing here is making use of a nondeterministic integer to decide which variant of `Rating` to return.

> **NOTE**: If we thought of this code as generating a random value, this function looks heavily biased.
> We'd overwhelmingly generate a `Three` because it's matching "all other integers besides 1 and 2."
> But Kani just see 3 meaningful possibilities, each of which is not treated any differently from each other.
> The "proportion" of integers does not matter.

## Bounding nondeterministic variables

You can use `kani::any()` for `[T; N]` (if implemented for `T`) because this array type has an exact and constant size.
But if you wanted a slice (`[T]`) up to size `N`, you can no longer use `kani::any()` for that.
Likewise, there is no implementation of `kani::any()` for more complex data structures like `Vec`.

The trouble with a nondeterministic vector is that you usually need to _bound_ the size of the vector, for the reasons we investigated in the [last chapter](./tutorial-loop-unwinding.md).
The `kani::any()` function does not have any arguments, and so cannot be given an upper bound.

This does not mean you cannot have a nondeterministic vector.
It just means you have to construct one.
Our example proof harness above constructs a nondeterministic `Inventory` of size `1`, simply by starting with the empty `Inventory` and inserting a nondeterministic entry. Alternatively, you can implement or derive `BoundedArbitrary` for your type, which behaves like `Arbitrary`, except it accepts a fixed bound for the collection. See the [bounded arbitrary](./reference/bounded_arbitrary.md) documentation for details.

### Exercise

Try writing a function to generate a (bounded) nondeterministic inventory (from the first example:)

```rust
fn any_inventory(bound: u32) -> Inventory {
   // fill in here
}
```

One thing you'll quickly find is that the bounds must be very small.
Kani does not (yet!) scale well to nondeterministic-size data structures involving heap allocations.
A proof harness like `safe_update` above, but starting with `any_inventory(2)` will probably take a couple of minutes to prove.

A hint for this exercise: you might choose two different behaviors, "size of exactly `bound`" or "size up to `bound`".
Try both!

A solution can be found in [`exercise_solution.rs`](https://github.com/model-checking/kani/blob/main/docs/src/tutorial/arbitrary-variables/src/exercise_solution.rs).

## Summary

In this section:

1. We saw how `kani::any()` will return "safe" values for each of the types Kani implements it for.
2. We saw how to implement `kani::Arbitrary` or just write a function to create nondeterministic values for other types.
3. We noted that some types cannot implement `kani::any()` as they need a bound on their size.
4. We did an exercise to generate nondeterministic values of bounded size for `Inventory`.
