# Non-deterministic variables

RMC is able to reason about programs and reachable paths by allowing users to assign non-deterministic values to 
certain variables. Since RMC is a bit-level model checker, this means that RMC considers that an unconstrained 
non-deterministic value represents all the possible bit-value combinations  assigned to the variable's memory 
position.

As a rust developer, this sounds a lot like `mem::transmute` operation, which is highly `unsafe`. And that's correct.

In this tutorial, we will show how to safely use non-deterministic assignments to generate valid symbolic variables 
that respect rust type invariant, as well as show how you can specify invariant for types that you create enabling 
creation of safe non-deterministic variables for those types.

## Safe non-deterministic variables

Let's say you are developing an inventory management tool, and you would like to verify that your API to manage 
items is correct. Here is a simple implementation of this API:

```rust
{{#include tutorial/arbitrary-variables/src/inventory.rs:inventory_lib}}
```

Now we would like to verify that no matter which combination of `id` and `quantity`, that a call to 
`Inventory::update()` followed 
by a 
call to `Inventory::get()` using the same id returns some 
value that is equal to the one we inserted:

```rust
{{#include tutorial/arbitrary-variables/src/inventory.rs:safe_update}}
```

In this harness, we use`rmc::any()` to generate `ProductId` and the new quantity. The `rmc::any()` is a **safe** API,
and it represents only valid values.

If we run this example, RMC verification will succeed, including the assertion that shows that the underlying 
`u32` variable  used to represent `NonZeroU32` cannot be zero, per its type invariant:

You can try it out by running the example under 
[`rmc-docs/src/tutorial/arbitrary-variables`](https://github.
com/model-checking/rmc/tree/main/rmc-docs/src/tutorial/arbitrary-variables/):

```cargo rmc --function safe_update``` 

## Unsafe non-deterministic variables

RMC also includes a **unsafe** method to generate unconstrained non-deterministic variables which do not take type invariant into consideration.
As any unsafe method in rust, users must be careful when using unsafe methods and ensure the right guardrails are 
put in place to avoid undesirable behavior.

That said, there may be cases where you want to verify your code taking into consideration that some inputs may 
contain invalid data.

Let's see what happens if we modify our verification harness to use the unsafe method `rmc::any_raw()` to generate 
the updated value.

```rust
{{#include tutorial/arbitrary-variables/src/inventory.rs:unsafe_update}}
```

We commented out the assertion that the underlying `u32` variable cannot be `0`, since this no longer holds. The RMC 
execution will now fail in the `inventory.get(&id).unwrap()` method call.

This is an interesting issue that emerges from how rustc optimizes the memory layout of `Option<NonZeroU32>`. The 
compiler is able to represent `Option<NonZeroU32>` using `32` bits by using the value `0` to represent `None`.  

You can try it out by running the example under
[`rmc-docs/src/tutorial/arbitrary-variables`](https://github.
com/model-checking/rmc/tree/main/rmc-docs/src/tutorial/arbitrary-variables/):

```cargo rmc --function unsafe_update``` 

## Safe non-deterministic for custom types

Now you would like to add a new structure to your library that allow users to represent a review rating, which can 
go from 0 to 5 stars. Let's say you add the following implementation:

```rust
{{#include tutorial/arbitrary-variables/src/rating.rs:rating_struct}}
```

The easiest way to allow users to create non-deterministic variables of the Rating type which represents values from 
0-5 stars is by implementing the `rmc::Invariant` trait.

The implementation only requires you to define a check to your structure that returns whether it's current value is 
valid or not. In our case, we have the following implementation:

```rust
{{#include tutorial/arbitrary-variables/src/rating.rs:rating_invariant}}
```

Now you can use `rmc::any()` to create valid Rating non-deterministic variables as shown by this harness:

```rust
{{#include tutorial/arbitrary-variables/src/rating.rs:verify_rating}}
```

You can try it out by running the example under
[`rmc-docs/src/tutorial/arbitrary-variables`](https://github.
com/model-checking/rmc/tree/main/rmc-docs/src/tutorial/arbitrary-variables/):

```cargo rmc --function check_rating``` 
