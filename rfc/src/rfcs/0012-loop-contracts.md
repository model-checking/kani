- **Feature Name:** Loop Contracts
- **Feature Request Issue:** [#3168](https://github.com/model-checking/kani/issues/3168)
- **RFC PR:** [#3167](https://github.com/model-checking/kani/pull/3167)
- **Status:** Under Review
- **Version:** 1
- **Proof-of-concept:** 

-------------------

## Summary

Loop contracts provide way to safely abstract loops of a program, typically
in order to accelerate the verification process, and remove the loop unwinding
bounds. The key idea is to over-approximate the possible set of program states,
while still being precise enough to be able to prove the desired property.

## User Impact

Loop contracts provide an interface for a verified, sound abstraction.
The goal for specifying loop contracts in the source code is two fold:

* Unbounded verification: Currently, proving correctness
  (i.e. assertions never fail) on programs with unbounded control flow (e.g. 
  loops with dynamic bounds) Kani requires unwinding loops for a large number of
  times, which is not always feasible. Loop contracts provide a way to abstract
  out loops, and hence remove the need for unwinding loops.
* Faster CI runs: In most cases, the provided contracts would also significantly
  improve Kani's verification time since all loops would be unrolled only to
  a single iteration.



Loop contracts are completely optional with no user impact if unused. This
RFC proposes the addition of new attributes, and functions, that shouldn't
interfere with existing functionalities.


## User Experience

A loop contract specifies the behavior of a loop as a boolean predicate
(loop invariants clauses) with certain frames conditions (loop modifies clauses)
that can be checked against the loop implementation, and used to abstract out
the loop in the verification process.

We illustrate the usage of loop contracts with an example.
Consider the following program:
```rs
fn simple_loop() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    while x > 1{
        x = x - 1;
    };

    assert!(x == 1);
}
```
The loop in the `simple_loop` function keep subtracting 1 from `x` until `x` is 1.
However, Kani currently needs to unroll the loop for `u64::MAX` number of times
to verify the assertion at the end of the program. 

With loop contracts, the user can specify the behavior of the loop as follows:
```rs
fn simple_loop_with_loop_contracts() {
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
The idea is, instead of exploring all possible branches of the loop, Kani only needs to
prove those branches reached from an arbitrary program state that satisfies the loop contracts,
after the execution of one iteration of the loop.

So, for loops without break statements, proving post-loops properties with loop contracts is
equivalent to proving the properties with the loop abstracted out as assuming the post-states
of the loops should satisfying the disjunction of the invariant and the negation of the loop guard.
The requirement of satisfying the negation of the loop guard comes from the fact that a path
exits loops without break statements must fail the loop guard.

For example, applying loop contracts in `simple_loop` function is equivalent to the following:
```rs
fn simple_loop_transformed() {
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
fn simple_loop_two_vars() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);
    let mut y: u64 = 1;

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_modifies(x)]
    while x > 1{
        x = x - 1;
    };

    assert!(x == 1);
    assert!(y == 1);
}
```
write to only `x` in the loop, hence the `modifies` clause contains only `x`.
Then when use the loop contracts to abstract the loop, Kani will only havoc the memory
location `x` and keep `y` unchanged. Note that if the `modifies` clause contains also
`y`, Kani will havoc both `x` and `y`, and hence violate the assertion `y == 1`.


Kani will also verify if all writing targets in the loop are included in the `modifies`
clause.


Note that the `modifies` clause is optional, and Kani will infer the write set if not
provided.


### Proof of termination
    
    Loop contracts also provide a way to prove the termination of the loop.
    Without the proof of termination, the loop contracts could lead to a false
    positive result. For example, consider the following program:

```rs
fn simple_loop_non_terminating() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    while true{
        x = x;
    };

    assert!(x >= 1);
}
```
After abstracting the loop, the loop will be transformed to no-op, and the assertion
`x >= 1` will be proved. However, the loop is actually an infinite loop, and the
assertion will never be reached.

For this reason, Kani will also require the user to provide a `decreases` clause that
specifies a decreasing expression to prove the termination of the loop. For example, in
```rs
fn simple_loop_terminating() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(x)]
    while x > 1{
        x = x - 1;
    };

    assert!(x >= 1);
}
```
, the `decreases` clause `#[kani::loop_decreases(x)]` specifies that the value of `x`
decreases at each iteration of the loop, and hence the loop will terminate.


## Detailed Design


Kani implements the functionality of loop contracts in three places.

1. Procedural macros `loop_invariant`, `loop_modifies`, and `loop_decreases`.
2. Code generation for builtin functions expanded from the above macros.
3. GOTO-level loop contracts using CBMC's contract language generated in
   `kani-compiler`.

### Procedural macros `loop_invariant`, `loop_modifies`, and `loop_decreases`.

The `loop_invariant` macro perform code generation for the loop invariant clause.
The generated code consists of two parts: 
1. a closure definition to wrap the loop invariant, which is an Boolean expression.
2. a call to a builtin function `kani_loop_invariant` at end of the loop.

As an example, in the above program, the following code will be generated for the
loop invariants clauses:

```rs
fn simple_loop_macro_expanded() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    while x > 1{
        x = x - 1;
        kani::kani_loop_invariant_begin_marker();
        let __kani_loop_invariant: bool = x >= 1;
        kani::kani_loop_invariant_end_marker();
    };

    assert!(x == 1);
}
```

Similarly, we generate calls to the corresponding builtin functions for `modifies` and `decreases`
clauses.


### Code Generation for Builtin Functions

When generating GOTO program from MIR, Kani will first scan for the placeholder function
calls `kani_loop_invariant_begin_marker` and `kani_loop_invariant_end_marker` in the MIR. 
Then Kani will generate the corresponding GOTO-level statement expression for all instructions
between the two placeholder function calls. At last, Kani will add the statement expression
to the loop latch---the jump back to the loop head.

The artifact `goto-instrument` in CBMC will extract the loop contracts from the named-subs
of the loop latch, and then apply and prove the extracted loop contracts.

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
condition checking for the transformed loop, which will save us from reinventing
the error-prone wheel. Second, the loop contracts synthesis tool we developed and
are developing are all based on GOTO level. Hence, doing the transformation in
the GOTO level will make the integration of loop contracts with the synthesis tool
easier.

## Open questions

## Future possibilities

<!-- For Developers -->

---
