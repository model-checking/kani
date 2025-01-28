# Loop Contracts

Loop contract are used to specify invariants for loops for the sake of extending Kani's *bounded proofs* to *unbounded proofs*.
A [loop invariant](https://en.wikipedia.org/wiki/Loop_invariant) is an expression that holds upon entering a loop and after every execution of the loop body.
It captures something that does not change about every step of the loop.

It is worth revisiting the discussion about [bounded proof](../../tutorial-loop-unwinding.md#bounded-proof) and
[loop unwinding](../../tutorial-loop-unwinding.md#loops-unwinding-and-bounds). In short, bounds on the number of times Kani unwinds loops also bound the size of inputs,
and hence result in a bounded proof.
Loop contracts are used to abstract out loops as non-loop blocks to avoid loop unwinding, and hence remove the bounds on the inputs.

Consider the following example:

``` Rust
fn simple_loop() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    while x > 1 {
        x = x - 1;
    };

    assert!(x == 1);
}
```

In this program, the loop repeatedly decrements `x` until it equals `1`. Because we haven't specify an upper bound for `x`, to verify this function,
Kani needs to unwind the loop for `u64::MAX` iterations, which is computationally expensive. Loop contracts allow us to abstract the loop behavior, significantly reducing the verification cost.

With loop contracts, we can specify the loop’s behavior using invariants. For example:

``` Rust
fn simple_loop_with_loop_contracts() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    while x > 1 {
        x = x - 1;
    };

    assert!(x == 1);
}
```

Here, the loop invariant `#[kani::loop_invariant(x >= 1)]` specifies that the condition `x >= 1` must hold true at the start of each iteration before the loop guard is
 checked. Once Kani verifies that the loop invariant is inductive, it will use the invariant to abstract the loop and avoid unwinding. 

 Now let's run the proof with loop contracts through kani:
 ``` bash
kani simple_loop_with_loop_contracts.rs  -Z loop-contracts
 ```


## Loop contracts for `while` loops

> **Syntax**
> 
> \#\[kani::loop_invariant\( [_Expression_](https://doc.rust-lang.org/reference/expressions.html) \)\]
> 
>  `while` [_Expression_](https://doc.rust-lang.org/reference/expressions.html)<sub>_except struct expression_</sub> [_BlockExpression_](https://doc.rust-lang.org/reference/expressions/block-expr.html)


An invariant contract `#[kani::loop_invariant(cond)]` accepts a valid Boolean expression `cond` over the variables visible at the same scope as the loop.

### Semantics
A loop invariant contract expands to several assumptions and assertions:
1. The invariant is asserted just before the first iteration.
2. The invariant is assumed on a non-deterministic state to model a non-deterministic iteration.
3. The invariant is finally asserted again to establish its inductiveness.

Mathematical induction is the working principle here. (1) establishes the base case for induction, and (2) & (3) establish the inductive case.
Therefore, the invariant must hold after the loop execution for any number of iterations. The invariant, together with the negation of the loop guard,
must be sufficient to establish subsequent assertions. If it is not, the abstraction is too imprecise and the user must supply a stronger invariant.

To illustrate the key idea, we show how Kani abstracts the loop in `simple_loop_with_loop_contracts` as a non-loop block:
``` Rust
assert!(x >= 1) // check loop invariant for the base case.
x = kani::any();
kani::assume(x >= 1);
if x > 1 {
    // proof path 1:
    //   both loop guard and loop invariant are satisfied.
    x = x - 1;
    assert!(x >= 1); // check that loop invariant is inductive.
    kani::assume(false) // block this proof path.
}
// proof path 2:
//   loop invariant is satisfied and loop guard is violated.
assert!(x == 1);
```
That is, we assume that we are in an arbitrary iteration after checking that the loop invariant holds for the base case. With the inductive hypothesis (`assume!(x >= 1);`),
we will either enter the loop (proof path 1) or leave the loop (proof path 2). We prove the two paths separately by killing path 1 with `assume!(false);`.
Note that all assertions after `assume!(false)` will be ignored as `false => p` can be deduced as `true` for any `p`.

In proof path 1, we prove properties inside the loop and at last check that the loop invariant is inductive.

In proof path 2, we prove properties after leaving the loop. As we leave the loop only when the loop guard is violated, the post condition of the loop can be expressed as
`!guard && inv`, which is `x <= 1 && x >= 1` in the example. The postcondition implies `x == 1`—the property we want to prove at the end of `simple_loop_with_loop_contracts`.


## Limitations

Loop contracts comes with the following limitations.

1. Only `while` loops are supported. The other three kinds of loops are not supported: [`loop` loops](https://doc.rust-lang.org/reference/expressions/loop-expr.html#infinite-loops)
   , [`while let` loops](https://doc.rust-lang.org/reference/expressions/loop-expr.html#predicate-pattern-loops), and [`for` loops](https://doc.rust-lang.org/reference/expressions/loop-expr.html#iterator-loops).
2. Kani infers *loop modifies* with alias analysis. Loop modifies are those variables we assume to be arbitrary in the inductive hypothesis, and should cover all memory locations that are written to during the execution of the loops. A proof will fail if the inferred loop modifies misses some targets written in the loops.
   We observed this happens when some fields of structs are modified by some other functions called in the loops.
3. Kani doesn't check if a loop will always terminate in proofs with loop contracts. So it could be that some properties are proved successfully with Kani but actually are unreachable due to the non-termination of some loops.
4. We don't check if loop invariants are side-effect free. A loop invariant with a side effect could lead to an unsound proof result. Make sure that the specified loop contracts are side-effect free.
