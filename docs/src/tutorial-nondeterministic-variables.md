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
If we run this, you'll notice that verification succeeds.

```bash
cargo kani --harness safe_update
```

`kani::any()` is safe Rust, and so Kani only implements it for types where type invariants are enforced.
For `NonZeroU32`, this means we never return a `0` value.
The assertion we wrote in this harness was just an extra check we added to demonstrate this fact.

## Other kinds of nondeterministic variables

There's nothing special about `kani::any()` except that Kani ships with implementations for a few types where we can guarantee safety.

Note that there is no implementation for `Vec<T>`, however.
The trouble with a nondeterministic vector is that you usually need to _bound_ the size of the vector, for the reasons we investigated in the [last chapter](./tutorial-loop-unwinding.md).
There are no arguments provided to `kani::any()` to indicate an upper bound.

You can use `kani::any()` for `[T; N]` (if implemented for `T`).
But this the type of an exactly-sized array.
Again, `kani::any()` does not have an implementation for an unsized slice (like `[T]`).

This does not mean you cannot have a nondeterministic vector.
It just means you have to construct one.
Our example proof harness above constructs a nondeterministic `Inventory` of size `1`, simply by starting with the empty `Inventory` and inserting a nondeterministic entry.

## Custom nondeterministic types

When you need nondeterministic variables of types that `kani::any()` cannot construct, our current recommendation is to simply write a function.

To generate a nondeterministic struct, you would just generate nondeterministic values for each of its fields.
For complex data structures, you can start with an empty one and add a (bounded) nondeterministic number of entries.
For an enum, you can make use of a simple trick:

```rust
{{#include tutorial/arbitrary-variables/src/rating.rs:rating_invariant}}
```

All we're doing here is making use of a nondeterministic integer to decide which variant of `Rating` to return.

> **NOTE**: If we thought of this code as generating a random value, this function looks sub-optimal.
> We'd overwhelming generate a `Three` because it's matching "all other integers besides 1 and 2."
> But Kani just see 3 meaningful possibilities, each of which is not treated any differently from each other.
> The "proportion" of integers does not matter.


## Exercise

Try writing a function to generate a nondeterministic inventory (from the first example:)

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
2. We did an exercise to generate nondeterministic values of bounded size for `Inventory`.
3. We saw a trick for how to safely generate a nondeterministic `enum`.
