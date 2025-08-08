# Loop Contracts

## Introduction

Loop contracts are used to specify invariants for loops for the sake of extending Kani's *bounded proofs* to *unbounded proofs*.
A [loop contract](https://en.wikipedia.org/wiki/Loop_invariant) is an expression that holds upon entering a loop and after every execution of the loop body.
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
    }

    assert!(x == 1);
}
```

In this program, the loop repeatedly decrements `x` until it equals `1`. Because we haven't specified an upper bound for `x`, to verify this function,
Kani needs to unwind the loop for `u64::MAX` iterations, which is intractable. Loop contracts allow us to abstract the loop behavior, significantly reducing the verification cost.

With loop contracts, we can specify the loop’s behavior using invariants. For example:

``` Rust
#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn simple_loop_with_loop_contracts() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    while x > 1 {
        x = x - 1;
    }

    assert!(x == 1);
}
```

Here, the loop contract `#[kani::loop_invariant(x >= 1)]` specifies that the condition `x >= 1` must hold true at the start of each iteration before the loop guard is
checked. Once Kani verifies that the loop contract is inductive, it will use the invariant to abstract the loop and avoid unwinding. 

Now let's run the proof with loop contracts through kani:
``` bash
kani simple_loop_with_loop_contracts.rs  -Z loop-contracts
```
The output reported by Kani on the example will be
```
...


Check 10: simple_loop_with_loop_contracts.loop_invariant_base.1
         - Status: SUCCESS
         - Description: "Check invariant before entry for loop simple_loop_with_loop_contracts.0"
         - Location: simple_while_loop.rs:15:5 in function simple_loop_with_loop_contracts

Check 11: simple_loop_with_loop_contracts.loop_assigns.1
         - Status: SUCCESS
         - Description: "Check assigns clause inclusion for loop simple_loop_with_loop_contracts.0"
         - Location: simple_while_loop.rs:15:5 in function simple_loop_with_loop_contracts

Check 13: simple_loop_with_loop_contracts.assigns.1
         - Status: SUCCESS
         - Description: "Check that x is assignable"
         - Location: simple_while_loop.rs:17:9 in function simple_loop_with_loop_contracts

Check 14: simple_loop_with_loop_contracts.loop_invariant_step.1
         - Status: SUCCESS
         - Description: "Check invariant after step for loop simple_loop_with_loop_contracts.0"
         - Location: simple_while_loop.rs:15:5 in function simple_loop_with_loop_contracts

Check 15: simple_loop_with_loop_contracts.loop_invariant_step.2
         - Status: SUCCESS
         - Description: "Check invariant after step for loop simple_loop_with_loop_contracts.0"
         - Location: simple_while_loop.rs:15:5 in function simple_loop_with_loop_contracts

...

SUMMARY:
 ** 0 of 99 failed

VERIFICATION:- SUCCESSFUL
Verification Time: 0.3897019s

Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```


## Syntax and Semantics

### Syntax
> 
> \#\[kani::loop_invariant\( [_Expression_](https://doc.rust-lang.org/reference/expressions.html) \)\]
> 
>  [_LoopExpression_](https://doc.rust-lang.org/reference/expressions/loop-expr.html#grammar-LoopExpression)


An invariant contract `#[kani::loop_invariant(cond)]` accepts a valid Boolean expression `cond` over the variables visible at the same scope as the loop.

### Semantics
A loop contract expands to several assumptions and assertions:
1. The invariant is asserted just before the first iteration.
2. The invariant is assumed on a non-deterministic state to model a non-deterministic iteration.
3. The invariant is finally asserted again to establish its inductiveness.

Mathematical induction is the working principle here. (1) establishes the base case for induction, and (2) & (3) establish the inductive case.
Therefore, the invariant must hold after the loop execution for any number of iterations. The invariant, together with the negation of the loop guard,
must be sufficient to establish subsequent assertions. If it is not, the abstraction is too imprecise and the user must supply a stronger invariant.

To illustrate the key idea, we show how Kani abstracts the loop in `simple_loop_with_loop_contracts` as a non-loop block:
``` Rust
assert!(x >= 1) // check loop contract for the base case.
x = kani::any();
kani::assume(x >= 1);
if x > 1 {
    // proof path 1:
    //   both loop guard and loop contract are satisfied.
    x = x - 1;
    assert!(x >= 1); // check that loop contract is inductive.
    kani::assume(false) // block this proof path.
}
// proof path 2:
//   loop contract is satisfied and loop guard is violated.
assert!(x == 1);
```
That is, we assume that we are in an arbitrary iteration after checking that the loop contract holds for the base case. With the inductive hypothesis (`kani::assume(x >= 1);`),
we will either enter the loop (proof path 1) or leave the loop (proof path 2). We prove the two paths separately by killing path 1 with `kani::assume(false);`.
Note that all assertions after `kani::assume(false)` will be ignored as `false => p` can be deduced as `true` for any `p`.

In proof path 1, we prove properties inside the loop and at last check that the loop contract is inductive.

In proof path 2, we prove properties after leaving the loop. As we leave the loop only when the loop guard is violated, the post condition of the loop can be expressed as
`!guard && inv`, which is `x <= 1 && x >= 1` in the example. The postcondition implies `x == 1`—the property we want to prove at the end of `simple_loop_with_loop_contracts`.

## Historic values and extra variables

### Historic values

We support two notations for historic values in loop contracts:
1. `on_entry(expr)` : The value of the `expr` before entering the loop.
2. `prev(expr)` : the value of `expr` in the previous iteration. Note that Kani will assert that the loop has at least one iteration if `prev` is used in loop contracts.

Example:
```Rust
#[kani::proof]
pub fn loop_with_old_and_prev() {
    let mut i = 100;
    #[kani::loop_invariant((i >= 2) && (i <= 100) && (i % 2 == 0) && (on_entry(i) == 100) && (prev(i) == i + 2))]
    while i > 2 {
        if i == 1 {
            break;
        }
        i = i - 2;
    }
    assert!(i == 2);
}
```

### Extra variables in `for` loop

There are two extra variables that can be used in loop contracts for `for` loops:
1. `kani::indexx` : the position (index) of the current iteration in the iterator.
2. `kani_iter_len` : the length of the iterator.

Note that the two extra variables above are only associated with the `for` loop right below the loop contract.

Example:

```Rust
#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 20));
    #[kani::loop_invariant( i == kani::index && i <= kani_iter_len && sum <= (kani::index as u32 * 29) )]
    for (i, j) in a.iter().enumerate() {
        sum = sum + (i as u32) + (*j as u32);
    }
    assert!(sum <= 290);
}
```

## loop contracts inside functions with contracts 
Kani supports using loop contracts together with function-contracts, as demonstrated in the following example:
``` Rust
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::requires(i>=2)]
#[kani::ensures(|ret| *ret == 2)]
pub fn has_loop(mut i: u16) -> u16 {
    #[kani::loop_invariant(i>=2)]
    while i > 2 {
        i = i - 1
    }
    i
}

#[kani::proof_for_contract(has_loop)]
fn contract_proof() {
    let i: u16 = kani::any();
    let j = has_loop(i);
}
```

When loop contracts and function-contracts are both enabled (by flags `-Z loop-contracts -Z function-contracts`), 
Kani automatically contracts (instead of unwinds) all loops in the functions that we want to prove contracts for.

## Loop modifies clauses: 
We allow users to manually specified the `loop_modifies` clauses for memory allocated addresses which can be modified inside the loop body.
The concept is very similar to the `__CPROVER_assigns` clause of CBMC (https://diffblue.github.io/cbmc/contracts-assigns.html).
However, in Kani, the CBMC target is replaced by three Rust types which can be used in the `loop_modifies` clauses:
1. `RawPtr`: We don't allow variable names as targets. Users must use pointers to them instead, which also allows checking modification using borrowed references and aliases.
```Rust
#[kani::proof]
fn main() {
    let mut i = 0;
    #[kani::loop_invariant(i <= 20)]
    #[kani::loop_modifies(&i as *const _)]
    while i < 20 {
        i = i + 1;
    }
}
```
2. `Reference`: Similar to RawPtr, but we also can use it to replace  `__CPROVER_object_whole(ptr-expr)`,
Example 
```Rust
#[kani::proof]
fn main() {
    let mut i = 0;
    let mut a: [u8; 20] = kani::any();
    #[kani::loop_invariant(i <= 20)]
    #[kani::loop_modifies(&i, &a)]
    while i < 20 {
        a[i] = 1;
        i = i + 1;
    }
}
```
3. `FatPtr (Slice)`: We use this to replace `__CPROVER_object_from(ptr-expr)`, and `__CPROVER_object_upto(ptr-expr, uint-expr)`.
```Rust
#[kani::proof]
fn main() {
    let mut i = 3;
    let mut a: [u8; 100] = kani::any();
    #[kani::loop_invariant(i >=3 && i <= 20)]
    #[kani::loop_modifies(&i , &a[3..20])]
    while i < 20 {
        a[i] = 1;
        i = i + 1;
    }
}
```
or

```Rust
use std::ptr::slice_from_raw_parts;
#[kani::proof]
fn main() {
    let mut i = 0;
    let mut a: [u8; 100] = kani::any();
    #[kani::loop_invariant(i <= 20)]
    #[kani::loop_modifies(&i , slice_from_raw_parts(a.as_ptr(), 20))]
    while i < 20 {
        a[i] = 1;
        i = i + 1;
    }
}
```

## Limitations

Loop contracts comes with the following limitations.

1. `while` loops, `loop` loops are supported. `for` loops are supported for array, slice, Iter, Vec, Range, StepBy, Chain, Zip, Map, and Enumerate. The other kinds of loops are not supported: [`while let` loops](https://doc.rust-lang.org/reference/expressions/loop-expr.html#predicate-pattern-loops).
2. Kani infers *loop modifies* with alias analysis. Loop modifies are those variables we assume to be arbitrary in the inductive hypothesis, and should cover all memory locations that are written to during 
   the execution of the loops. A proof will fail if the inferred loop modifies misses some targets written in the loops.
   We observed this happens when some fields of structs are modified by some other functions called in the loops.
3. Kani doesn't check if a loop will always terminate in proofs with loop contracts. So it could be that some properties are proved successfully with Kani but actually are unreachable due to the 
   non-termination of some loops.
4. We don't check if loop contracts are side-effect free. A loop contract with a side effect could lead to an unsound proof result. Make sure that the specified loop contracts are side-effect free.
