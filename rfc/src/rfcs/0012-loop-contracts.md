- **Feature Name:** Loop Contracts
- **Feature Request Issue:** 
- **RFC PR:** 
- **Status:** Under Review
- **Version:** 1
- **Proof-of-concept:** 

-------------------

## Summary

Loop contracts provide way to safely abstract loops of a program, typically
in order to accelerate the verification process, and remove the loop unwinding
bounds. The key idea is to overapproximate the possible set of program states,
while still being precise enough to be able to prove the desired property.

## User Impact

Loop contracts provide an interface for a verified, sound abstraction.
The goal for specifying loop contracts in the source code is three fold:

* Unbounded verification: Currently Kani does not support proving correctness
  (i.e. assertions never fail) on programs with unbounded control flow (e.g. 
  loops with dynamic bounds). Kani unrolls all unbounded loops until a few
  iterations and then verifies this unrolled program — it thus provides a much
  weaker guarantee on correctness.
* Faster CI runs: These contracts, when provided, would also significantly
  improve Kani's verification time since all loops would be unrolled only to
  a single iteration, as opposed to a small number of iterations which is
  Kani's current behavior.



Loop contracts are completely optional with no user impact if unused. This
RFC proposes the addition of new attributes, and functions, that shouldn't
interfere with existing functionalities.


## User Experience

A loop contract specifies the behavior of a loop as a predicate that
can be checked against the loop implementation, and used to abstract out
the loop in the verification process.

We illustrate the usage of loop contracts with an example.
Consider the following program:
```rs
fn main() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    while x > 1{
        x = x - 1;
    };

    assert!(x == 1);
}
```
The loop in the `main` function keep subtracting 1 from `x` until `x` is 1.
However, Kani currently needs to unroll the loop for `u64::MAX` number of times
to verify the assertion at the end of the program. 

With loop contracts, the user can specify the behavior of the loop as follows:
```rs
fn main() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    while x > 1{
        x = x - 1;
    };

    assert!(x == 1);
}
```
The loop invariant clause `#[kani::loop_invariant(x >= 1)]` specifies the loop
invariants that must hold at the beginning of each iteration of the loop right before
checking the loop guard.

In this case, Kani verifies that the loop invariant `x >= 1` is inductive, i.e.,
`x` is always greater than or equal to 1 at each iteration before checking `x > 1`.


Also, once Kani proved that the loop invariant is inductive, it can safely use the loop
invariants to abstract the loop out of the verification process.
The idea is, instead of exploring all possible branches of the loop, Kani can safely
substitute the loops with any of program states that satisfy the loop invariants and
the negation of the loop guard. The requirement of satisfying the negation of the loop
guard comes from the fact that a path exits the loop must fail the loop guard.
After the loop is abstracted, the program will be equivalent to the following:
```rs
fn main() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    x = kani::any(); // Arbitrary program state that
    kani::assume( !(x > 1) && x >= 1); // satisfies !`guard` && `inv` 

    assert!(x == 1);
}
```
The assumption above is actually equivalent to `x == 1`, hence the assertion at the end
of the program is proved.

### Write Sets and Havocking

For those memory locations that are not modified in the loop, loop invariants state
that they stay unchanged throughout the loop are inductive. In other words, Kani should
only havoc the memory locations that are modified in the loop. This is achieved by
specifying the `modifies` clause for the loop. For example, the following program:
```rs
fn main() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);
    let mut y: u64 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_modifies(x)]
    while x > 1{
        x = x - 1;
    };

    assert!(x == 1);
}
```
write to only `x` in the loop, hence the `modifies` clause contains only `x`.
Then when use the loop contracts to abstract the loop, Kani will only havoc the memory
location `x` and keep `y` unchanged. 


Kani will also verify if all writing targets in the loop are included in the `modifies`
clause.


Note that the `modifies` clause is optional, and Kani will infer the write set if not
provided.

## Detailed Design


Kani implements the functionality of loop contracts in three places.

1. Procedural macros `loop_invariant` and `loop_modifies`.
2. Code generation for builtin functions expanded from the above two macros.
3. GOTO-level loop contracts using CBMC's contract language generated in
   `kani-compiler` for `loop-modifies` clauses.

### Procedural macros `loop_invariant` and `loop_modifies`.

The `loop_invariant` macro perform code generation for the loop invariant clause.
The generated code consists of two parts: 
1. a closure definition to wrap the loop invariant, which is an boolean expression.
2. a call to a builtin function `kani_loop_invariant` at end of the loop.

As an example, in the above program, the following code will be generated for the
loop invariants clauses:

```rs
fn main() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    let _loop_invariant_closure = || x >= 1;
    while x > 1{
        x = x - 1;
        kani_loop_invariant_begin();
        _loop_invariant_closure()
        kani_loop_invariant_end();
    };

    assert!(x == 1);
}
```

Similarly, we generate a call to the builtin function `kani_loop_modifies` for modifies
clauses.


### Code Generation for Builtin Functions

When generating GOTO program from MIR, Kani will first scan for the placeholder function
calls `kani_loop_invariant_begin` and `kani_loop_invariant_end` in the MIR. Then Kani
will generate the corresponding GOTO-level statement expression for all instructions
between the two placeholder function calls. At last, Kani will add the statement expression
to the loop latch---the jump back to the loop head.

The artifact `goto-instrument` in CBMC will extract the loop contracts from the named-subs
of the loop latch, and the apply and prove the extracted loop contracts.

Similarly, Kani will add the `modifies` targets into the named-subs of the loop latch for
CBMC to extract and prove the loop contracts.


### GOTO-Level Havocing

The ordinary havocing in CBMC is not aware of the type constraints of Rust type.
Hence, we will use customized havocing functions for modifies targets. In detail,
Kani will generate code for the definition of corresponding `kani::any()` functions
for each modifies target. Then Kani will create a map from the modifies target to the
the name of its `kani::any()` function, and add the map to the loop latch too.

On the CBMC site, `goto-instrument` will extract the map and instrument the customized
havocing functions for the modifies targets.

## Rationale and alternatives



### Rust-Level Transformation vs CBMC 

Besides transforming the loops in GOTO level using `goto-instrument`,
we could also do the transformation in Rust level using procedural macros, or
in MIR level.

There are two reasons we prefer the GOTO-level transformation.
First, `goto-instrument` is a mature tool that can correctly instrument the frame
condition checking for the transformed loop, which will safe us from reinventing
the error-prone wheel. Second, the loop contracts synthesis tool we developed and
are developing are all based on GOTO level. Hence, doing the transformation in
the GOTO level will make the integration of loop contracts with the synthesis tool
easier.

## Open questions

<!-- For Developers -->

- The idea of using closure to wrap the loop invariant is a bit hacky. It is not
  clear what behavior of the loop will move the variables in the closure, and hence
  invalidate the closure. Is there a better way to do this?
<!-- 
- Is there any part of the design that you expect to resolve through the RFC process?
- What kind of user feedback do you expect to gather before stabilization? How will this impact your design? 
-->

## Future possibilities

<!-- For Developers -->

---
