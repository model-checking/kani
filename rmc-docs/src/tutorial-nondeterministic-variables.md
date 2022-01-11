# Non-deterministic variables

RMC is able to reason about programs and reachable paths by allowing users to assign non-deterministic values to 
certain variables. Since RMC is a bit-level model checker, this means that RMC considers that an unconstrained 
non-deterministic value represents all the possible bit-value combinations  assigned to the variable's memory 
position.

As a rust developer, this sounds a lot like `mem::transmute` operation, which is highly `unsafe`. And that's correct.

In this tutorial, we will show how to safely use non-deterministic assignments to generate valid symbolic variables 
that respect rust type invariant, as well as show how you can specify invariant for types that you create.

## Safe non-deterministic variables

Let's say you are developing an inventory management tool, and you would like to verify that your API to manage 
items is correct. Here is a simple implementation of this API:

- `rmc::any`: This is a **safe** API, that allows you to create a non-deterministic variable of a type T. I.e., it will generate a variable that represents all possible bit-value combinations except for those that violate the type invariant.


- `rmc::any_raw`: