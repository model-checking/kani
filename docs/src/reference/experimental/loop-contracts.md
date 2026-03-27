# Loop Contracts

## Introduction

Loop contracts are used to specify invariants for loops for the sake of extending Kani's *bounded proofs* to *unbounded proofs*.
A [loop invariant](https://en.wikipedia.org/wiki/Loop_invariant) is an expression that holds upon entering a loop and after every execution of the loop body.
Loop contracts are composed of one or more loop invariants as well as optional `loop_modifies` attributes.
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

Here, the loop invariant `#[kani::loop_invariant(x >= 1)]` specifies that the condition `x >= 1` must hold true at the start of each iteration before the loop guard is
checked. Once Kani verifies that the loop invariant is inductive, it will use the invariant to abstract the loop and avoid unwinding. 

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
That is, we assume that we are in an arbitrary iteration after checking that the loop invariant holds for the base case. With the inductive hypothesis (`kani::assume(x >= 1);`),
we will either enter the loop (proof path 1) or leave the loop (proof path 2). We prove the two paths separately by killing path 1 with `kani::assume(false);`.
Note that all assertions after `kani::assume(false)` will be ignored as `false => p` can be deduced as `true` for any `p`.

In proof path 1, we prove properties inside the loop and at last check that the loop contract is inductive.

In proof path 2, we prove properties after leaving the loop. As we leave the loop only when the loop guard is violated, the post condition of the loop can be expressed as
`!guard && inv`, which is `x <= 1 && x >= 1` in the example. The postcondition implies `x == 1`—the property we want to prove at the end of `simple_loop_with_loop_contracts`.

### Partial correctness vs. total correctness

The verification steps above establish what is known as *partial correctness*: **if** the loop
terminates, the result is correct. This is the standard guarantee provided by loop invariants alone.

To establish *total correctness* — that the loop both terminates **and** produces the correct
result — you additionally need a [decreases clause](#decreases-clauses-termination-proofs) that
proves the loop makes progress toward termination on every iteration.

In formal terms, the four steps of a complete loop correctness argument are:

1. **Establishment** (base case): The invariant holds before the first iteration.
2. **Preservation** (inductive step): If the invariant and the loop guard both hold at the start of
   an iteration, the invariant still holds at the end of that iteration.
3. **Postcondition**: When the loop exits (guard is false) and the invariant holds, the desired
   result follows.
4. **Termination**: A *variant* (also called *decrementing function* or *ranking function*) strictly
   decreases on every iteration and is bounded from below, guaranteeing the loop eventually exits.

Kani checks steps 1–3 automatically via `#[kani::loop_invariant]`. Step 4 is checked when you add
`#[kani::loop_decreases]`. Without step 4, Kani's proofs are *partial* — they are sound assuming
the loop terminates, but cannot rule out non-termination.

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

### `kani::index` variable in `for` loop

Kani provides an extra variable: `kani::index` that can be used in loop contracts of `for` loops.
`kani::index` presents the position (index) of the current iteration in the iterator 
and is only associated with the `for` loop that immediately follows the loop contract.

Example:

```Rust
#[kani::proof]
fn forloop() {
    let mut sum: u32 = 0;
    let a: [u8; 10] = kani::any();
    kani::assume(kani::forall!(|i in (0,10)| a[i] <= 20));
    #[kani::loop_invariant(sum <= (kani::index as u32 * 20) )]
    for x in a {
        sum = sum + x as u32;
    }
    assert!(sum <= 200);
}
```

## Loop contracts inside functions with contracts 
Kani supports using loop contracts together with function contracts, as demonstrated in the following example:
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

When loop contracts and function contracts are both enabled (by flags `-Z loop-contracts -Z function-contracts`), 
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

## Decreases clauses (Termination proofs)

### Why termination matters

Without a proof of termination, Kani may successfully verify properties that are actually unreachable
due to non-terminating loops. For example:

```Rust
#[kani::proof]
fn unsound_without_termination() {
    let mut x: u64 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    while true {
        x = x;  // infinite loop — x never changes
    }

    // This assertion is "proved" because the loop abstraction
    // assumes we exit the loop, but we never actually do.
    assert!(x >= 1);
}
```

After abstracting the loop, the assertion appears to hold. But the loop never terminates, so the
assertion is unreachable. A decreases clause prevents this class of unsound results by requiring
the user to prove that the loop makes progress toward termination.

### Background: Floyd's method

The technique used here was proposed by Robert Floyd in his seminal 1967 paper
[*Assigning Meaning to Programs*](https://people.eecs.berkeley.edu/~necula/Papers/FloydMeaning.pdf).
The idea is to find a *variant* (also called a *ranking function* or *termination measure*) — a
value that:

1. Is bounded from below (e.g., non-negative integers), and
2. Strictly decreases at each iteration of the loop.

Since the value cannot decrease forever (it must eventually hit the lower bound), the loop must terminate.

### Syntax

```
#[kani::loop_decreases(expr)]
#[kani::loop_decreases(expr1, expr2, ..., exprN)]
```

The decreases clause accepts one or more arithmetic expressions over the variables visible at the
same scope as the loop. It must be placed alongside the loop invariant, before the loop expression.

Multiple expressions form an ordered tuple compared using
[lexicographic ordering](https://en.wikipedia.org/wiki/Lexicographic_order) (see below).

### Basic example

```Rust
#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

#[kani::proof]
fn simple_decreases() {
    let mut x: u8 = kani::any_where(|i| *i >= 1);

    #[kani::loop_invariant(x >= 1)]
    #[kani::loop_decreases(x)]
    while x > 1 {
        x = x - 1;
    }

    assert!(x == 1);
}
```

Here, `x` strictly decreases at each iteration and is bounded below by `1` (the loop guard ensures
`x > 1`), so the loop must terminate.

### Multi-dimensional decreases (lexicographic ordering)

Some loops cannot be proved terminating with a single expression. For example, a loop with two
counters where the inner counter resets may need a tuple `(outer, inner)`. CBMC compares tuples
lexicographically: if the first element decreases, the remaining elements are free to increase or
stay the same. Only if the first element stays the same must the second element decrease, and so on.

This is the same principle used to prove termination of the
[Ackermann function](https://en.wikipedia.org/wiki/Ackermann_function), which requires a
two-dimensional decreases clause `(m, n)`.

```Rust
#[kani::proof]
fn nested_counters() {
    let n: u8 = kani::any_where(|i| *i >= 1 && *i <= 5);
    let mut i: u8 = 0;
    let mut j: u8 = 0;

    #[kani::loop_invariant(i <= n && j <= n)]
    #[kani::loop_decreases(n - i, n - j)]
    while i < n {
        if j < n {
            j += 1;
        } else {
            i += 1;
            j = 0;
        }
    }

    assert!(i == n);
}
```

When `j < n`, the first component `n - i` stays the same but the second component `n - j` decreases.
When `j >= n`, the first component `n - i` decreases (and `n - j` resets — which is fine because
the first component already decreased).

### Semantics

The decreases clause is checked by CBMC's `goto-instrument` through the following instrumentation:

1. At the beginning of the loop body (after the loop guard is satisfied), the current value of the
   measure is recorded in a temporary variable (`old_measure`).
2. At the end of the loop body (before the back-edge), the new value of the measure is recorded
   (`new_measure`).
3. An assertion checks that `new_measure < old_measure` (strict decrease).

For multi-dimensional decreases `(e1, e2, ..., en)`, the strict arithmetic comparison is extended
to a strict lexicographic comparison: the tuple `(new_e1, ..., new_en)` must be lexicographically
less than `(old_e1, ..., old_en)`.

Note that CBMC does not require the measure to be non-negative at all times. It only requires that
the loop does not execute again when the measure would go below zero. In other words, if the measure
becomes negative but the loop guard is false, that is acceptable.

### Interaction with loop invariants

Decreases clauses work in conjunction with loop invariant clauses. The invariant establishes the
context (the inductive hypothesis) under which the decreases check is performed. If a decreases
clause is annotated on a loop without an invariant clause, the weakest possible invariant (`true`)
is used to model an arbitrary iteration.

In practice, you almost always want both: the invariant constrains the state space, and the
decreases clause proves progress within that constrained space.

### Decreases clause limitations and comparison with other tools

The following limitations apply to decreases clauses in Kani. We compare with
[Dafny](https://dafny.org/latest/OnlineTutorial/Termination) and
[Verus](https://verus-lang.github.io/verus/guide/) to provide context.

1. **Integer-only measures.** Kani (via CBMC) only supports integer-typed expressions in decreases
   clauses. Dafny additionally supports sequences (compared by length), sets (compared by strict
   subset inclusion), booleans, and references as termination measures. Verus similarly supports
   richer types. In Kani, if you need to use a non-integer measure, you must manually map it to an
   integer expression (e.g., use `slice.len() - i` instead of the slice itself).

2. **No automatic inference.** Kani does not automatically guess a decreases clause. Dafny
   automatically infers `B - A` when it sees a loop guard of the form `A < B`, and infers parameter
   decreases for simple recursive functions. In Kani, the user must always provide an explicit
   decreases clause.

3. **No recursive function termination.** Kani's decreases clauses only apply to loops, not to
   recursive functions. CBMC itself does not support termination proofs for recursion (as noted in
   CBMC PR [#6236](https://github.com/diffblue/cbmc/pull/6236)). Dafny and Verus both support
   `decreases` annotations on recursive and mutually recursive functions. In Kani, recursive
   functions must be handled by other means (e.g., unwinding bounds).

4. **No `decreases *` escape hatch.** Dafny provides `decreases *` to explicitly opt out of
   termination checking for loops or functions where termination is unknown or intentionally
   absent (e.g., the Collatz conjecture, stream processors). Kani has no equivalent — if you
   cannot prove termination, simply omit the decreases clause. The loop will still be abstracted
   by the invariant, but without a termination guarantee.

5. **No side-effect checking.** Kani does not verify that decreases clause expressions are
   side-effect free. A decreases clause with side effects (e.g., mutation of variables) could lead
   to unsound results. The user must ensure the expression is pure.

6. **Strict decrease only.** CBMC checks `new_measure < old_measure` using strict integer
   comparison. There is no support for custom well-founded orderings or user-defined comparison
   operators. The measure must map to integer arithmetic.

## Worked example: Binary search

Binary search is a classic algorithm that is deceptively tricky to get right. It is an excellent
case study for loop contracts because the invariant, postcondition, and termination argument are
all non-trivial and interdependent.

```Rust
#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn binary_search(arr: &[i32; 5], target: i32) -> Option<usize> {
    let mut lo: usize = 0;
    let mut hi: usize = arr.len();

    #[kani::loop_invariant(lo <= hi && hi <= arr.len())]
    #[kani::loop_decreases(hi - lo)]
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if arr[mid] == target {
            return Some(mid);
        } else if arr[mid] < target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    None
}
```

Let's walk through the four correctness steps:

**1. Establishment.** Initially `lo = 0` and `hi = arr.len()`, so `0 <= lo <= hi <= arr.len()`
holds trivially.

**2. Preservation.** We need to show that if `lo <= hi <= arr.len()` and `lo < hi` (the guard),
then after the loop body, the invariant still holds. The key insight is that
`mid = lo + (hi - lo) / 2`, so `lo <= mid < hi`. In the `arr[mid] < target` branch, `lo` becomes
`mid + 1`, which is at most `hi`. In the `arr[mid] >= target` branch, `hi` becomes `mid`, which is
at least `lo`. Either way, `lo <= hi <= arr.len()` is preserved.

**3. Postcondition.** When the loop exits, `lo >= hi`. Combined with the invariant `lo <= hi`, we
get `lo == hi`. If the target was not found via early return, the search space is empty.

**4. Termination.** The decreases clause is `hi - lo`. The invariant guarantees this is
non-negative. In each branch, either `lo` increases (to `mid + 1 > lo`) or `hi` decreases (to
`mid < hi`). Either way, `hi - lo` strictly decreases. Note how the termination argument *depends
on the invariant* — without knowing `lo <= hi`, we couldn't guarantee the measure is non-negative.

This example illustrates a general principle: the invariant and the decreases clause work together.
The invariant constrains the state space, and the decreases clause proves progress within that
constrained space.

## Practical guidance

### Choosing a loop invariant

A loop invariant captures the core reason why the loop works. Think of it as an explanation of the
loop's progress that holds at every iteration boundary. A good approach:

- **Start from the postcondition.** What do you need to be true when the loop exits? The invariant
  should generalize this to any intermediate iteration.
- **Include bounds.** Almost every loop invariant needs bounds on the loop variable
  (e.g., `i <= n`). Without bounds, Kani's havoc step can assign arbitrary values.
- **Include the relationship between variables.** If the loop maintains a relationship between
  variables (e.g., `sum == a[0] + a[1] + ... + a[i-1]`), that relationship belongs in the
  invariant.

### Common mistakes

**Invariant too weak.** This is the most common error. Symptoms:
- The *postcondition step* fails: Kani reports a failing assertion after the loop, because the
  invariant combined with `!guard` is not strong enough to imply the desired property.
- The *preservation step* fails: the invariant doesn't constrain the state enough to prove itself
  inductive.

**Invariant too strong.** Symptoms:
- The *establishment step* fails: the invariant doesn't hold before the first iteration.
- The *preservation step* fails: the loop body cannot maintain the overly strong condition.

**Wrong decreases clause.** Symptoms:
- Kani reports `FAILURE` on `"Check variant decreases after step for loop"`. This means the
  measure did not strictly decrease on some iteration. Common causes:
  - The measure doesn't capture the right quantity (e.g., using `i` when you should use `n - i`).
  - The loop body has a path that doesn't make progress (e.g., a `continue` without modifying the
    measure).
  - The measure is a constant expression.

### Tips for decreases clauses

- For `while i < n` loops that increment `i`, the natural measure is `n - i`.
- For `while x > 0` loops that decrement `x`, the natural measure is `x`.
- For nested loops, each loop needs its own decreases clause. The inner loop's measure must
  decrease independently of the outer loop.
- If a single expression doesn't work, try a multi-dimensional decreases clause. Think about which
  variable decreases "most of the time" (put it first) and which decreases "within" the first
  (put it second).

## Limitations

Loop contracts comes with the following limitations.

1. `while` loops, `loop` loops are supported. `for` loops are supported for array, slice, Iter, Vec, Range, StepBy, Chain, Zip, Map, and Enumerate. The other kinds of loops are not supported: [`while let` loops](https://doc.rust-lang.org/reference/expressions/loop-expr.html#predicate-pattern-loops).
2. Kani infers *loop modifies* with alias analysis. Loop modifies are those variables we assume to be arbitrary in the inductive hypothesis, and should cover all memory locations that are written to during 
   the execution of the loops. A proof will fail if the inferred loop modifies misses some targets written in the loops.
   We observed this happens when some fields of structs are modified by some other functions called in the loops.
3. We don't check if loop contracts (invariants, decreases clauses) are side-effect free. A loop contract with a side effect could lead to an unsound proof result. Make sure that the specified loop contracts are side-effect free.
4. Decreases clauses only support integer-typed expressions. See the [decreases clause limitations](#decreases-clause-limitations-and-comparison-with-other-tools) section for a detailed comparison with other verification tools.
