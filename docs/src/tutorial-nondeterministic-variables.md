# Nondeterministic variables

Kani is able to reason about programs and their execution paths by allowing users to assign nondeterministic (i.e., symbolic) values to  certain variables.
Since Kani is a bit-level model checker, this means that Kani considers that an unconstrained nondeterministic value represents all the possible bit-value combinations assigned to the variable's memory contents.

As a Rust developer, this sounds a lot like the `mem::transmute` operation, which is highly `unsafe`.
And that's correct.

In this tutorial, we will show how to:
 1. Safely use nondeterministic assignments to generate valid symbolic variables that respect Rust's type invariants.
 2. Unsafely use nondeterministic assignments to generate unconstrained symbolic variables that do not respect Rust's type invariants.
 2. Specify invariants for types that you define, enabling the creation of safe nondeterministic variables for those types.

## Safe nondeterministic variables

Let's say you're developing an inventory management tool, and you would like to verify that your API to manage items is correct.
Here is a simple implementation of this API:

```rust,noplaypen
{{#include tutorial/arbitrary-variables/src/inventory.rs:inventory_lib}}
```

Now we would like to verify that, no matter which combination of `id` and `quantity`, a call to `Inventory::update()` followed by a call to `Inventory::get()` using the same `id` returns some value that's equal to the one we inserted:

```rust,noplaypen
{{#include tutorial/arbitrary-variables/src/inventory.rs:safe_update}}
```

In this harness, we use `kani::any()` to generate the new `id` and `quantity`.
`kani::any()` is a **safe** API function, and it represents only valid values.

If we run this example, Kani verification will succeed, including the assertion that shows that the underlying `u32` variable  used to represent `NonZeroU32` cannot be zero, per its type invariant:

You can try it out by running the example under
[`arbitrary-variables`](https://github.com/model-checking/kani/tree/main/docs/src/tutorial/arbitrary-variables/):

```
cargo kani --harness safe_update
```

## Unsafe nondeterministic variables

Kani also includes an **unsafe** method to generate unconstrained nondeterministic variables which do not take type invariants into consideration.
As with any unsafe method in Rust, users have to make sure the right guardrails are put in place to avoid undesirable behavior.

That said, there may be cases where you want to verify your code taking into consideration that some inputs may contain invalid data.

Let's see what happens if we modify our verification harness to use the unsafe method `kani::any_raw()` to generate the updated value.

```rust,noplaypen
{{#include tutorial/arbitrary-variables/src/inventory.rs:unsafe_update}}
```

We commented out the assertion that the underlying `u32` variable cannot be `0`, since this no longer holds.
The verification will now fail showing that the `inventory.get(&id).unwrap()` method call can panic.

This is an interesting issue that emerges from how `rustc` optimizes the memory layout of `Option<NonZeroU32>`.
The compiler is able to represent `Option<NonZeroU32>` using `32` bits by using the value `0` to represent `None`.

You can try it out by running the example under [`arbitrary-variables`](https://github.com/model-checking/kani/tree/main/docs/src/tutorial/arbitrary-variables/):

```
cargo kani --harness unsafe_update
```

## Safe nondeterministic variables for custom types

Now you would like to add a new structure to your library that allow users to represent a review rating, which can go from 0 to 5 stars.
Let's say you add the following implementation:

```rust,noplaypen
{{#include tutorial/arbitrary-variables/src/rating.rs:rating_struct}}
```

The easiest way to allow users to create nondeterministic variables of the Rating type which represents values from 0-5 stars is by implementing the `kani::Invariant` trait.

The implementation only requires you to define a check to your structure that returns whether its current value is valid or not.
In our case, we have the following implementation:

```rust,noplaypen
{{#include tutorial/arbitrary-variables/src/rating.rs:rating_invariant}}
```

Now you can use `kani::any()` to create valid nondeterministic variables of the Rating type as shown in this harness:

```rust,noplaypen
{{#include tutorial/arbitrary-variables/src/rating.rs:verify_rating}}
```

You can try it out by running the example under
[`arbitrary-variables`](https://github.com/model-checking/kani/tree/main/docs/src/tutorial/arbitrary-variables/):

```
cargo kani --harness check_rating
```
