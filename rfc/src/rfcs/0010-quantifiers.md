- **Feature Name:** Quantifiers
- **Feature Request Issue:** [#2546](https://github.com/model-checking/kani/issues/2546) and [#836](https://github.com/model-checking/kani/issues/836)
- **RFC PR:** [#](https://github.com/model-checking/kani/pull/)
- **Status:** Unstable
- **Version:** 1

-------------------

## Summary

Quantifiers are like logic-level loops and a powerful reasoning helper, which allow us to express statements about objects in a domain that satisfy a given condition. There are two primary quantifiers: the existential quantifier (∃) and the universal quantifier (∀).

1. The existential quantifier (∃): represents the statement "there exists." We use to express that there is at least one object in the domain that satisfies a given condition. For example, "∃x P(x)" means "there exists an x such that P(x) is true."

2. The universal quantifier (∀): represents the statement "for all" or "for every." We use it to express that a given condition is true for every object in the domain. For example, "∀x P(x)" means "for every x, P(x) is true."

## User Impact

Quantifiers enhance users' expressive capabilities, enabling the verification of more intricate properties in a concise manner.
Rather than exhaustively listing all elements in a domain, quantifiers enable users to make statements about the entire domain at once. This compact representation is crucial when dealing with large or unbounded inputs. Quantifiers also facilitate abstraction and generalization of properties. Instead of specifying properties for specific instances, quantified properties can capture general patterns and behaviors that hold across different objects in a domain. Additionally, by replacing loops in the specification with quantifiers, Kani can encode the properties more efficiently within the specified bounds, making the verification process more manageable and computationally feasible.

This new feature doesn't introduce any breaking changes to users. It will only allow them to write properites using the existential (∃) and universal (∀) quantifiers.

## User Experience

We propose a syntax inspired by [Prusti](https://viperproject.github.io/prusti-dev/user-guide/syntax.html?highlight=quanti#quantifiers). Quantifiers must have only one bound variable (restriction imposed today by CBMC). The syntax of existential (i.e., `kani::exists`) and universal (i.e., `kani::forall`) quantifiers are:

```rust
kani::exists(|<bound variable>: <bound variable type>| <range> && <Boolean expression>)
kani::forall(|<bound variable>: <bound variable type>| <range> ==> <Boolean expression>)
```

where `<range>` is an expression of the form

```rust
<lower bound> <= <bound variable> && <bound variable> < <upper bound>
```

where `lower bound` and `upper bound` are constants. The bound predicates could be strict (e.g., `<lower bound> < <bound variable>`), or non-strict (e.g., `<upper bound> <= <bound variable>`), but both the bounds must be **constants**. CBMC's SAT backend only supports bounded quantification under **constant** lower and upper bounds.

Consider the following example adapted from the documentation for the [from_raw_parts](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.from_raw_parts) function:

```rust
use std::ptr;
use std::mem;

#[kani::proof]
fn main() {
    let v = vec![kani::any::<usize>(); 100];

    // Prevent running `v`'s destructor so we are in complete control
    // of the allocation.
    let mut v = mem::ManuallyDrop::new(v);

    // Pull out the various important pieces of information about `v`
    let p = v.as_mut_ptr();
    let len = v.len();
    let cap = v.capacity();

    unsafe {
        // Overwrite memory
        for i in 0..len {
            *p.add(i) += 1;
        }

        // Put everything back together into a Vec
        let rebuilt = Vec::from_raw_parts(p, len, cap);
    }
}
```

Given the `v` vector has non-deterministic values, there are potential arithmetic overflows that might happen in the for loop. So we need to constrain all values of the array. We may also want to check all values of `rebuilt` after the operation. Without quantifiers, we might be tempt to use loops as follows:

```rust
use std::ptr;
use std::mem;

#[kani::proof]
fn main() {
    let original_v = vec![kani::any::<usize>(); 100];
    let v = original_v.clone();
    for i in 0..v.len() {
        kani::assume(v[i] < 5);
    }

    // Prevent running `v`'s destructor so we are in complete control
    // of the allocation.
    let mut v = mem::ManuallyDrop::new(v);

    // Pull out the various important pieces of information about `v`
    let p = v.as_mut_ptr();
    let len = v.len();
    let cap = v.capacity();

    unsafe {
        // Overwrite memory
        for i in 0..len {
            *p.add(i) += 1;
        }

        // Put everything back together into a Vec
        let rebuilt = Vec::from_raw_parts(p, len, cap);
        for i in 0..len {
            assert_eq!(rebuilt[i], original_v[i]+1);
        }
    }
}
```

This, however, might unnecessary increase the complexity of the verication process. We can achieve the same effect using quantifiers as shown below.

```rust
use std::ptr;
use std::mem;

#[kani::proof]
fn main() {
    let original_v = vec![kani::any::<usize>(); 3];
    let v = original_v.clone();
    kani::assume(kani::forall(|i: usize| (i < v.len()) ==> (v[i] < 5)));

    // Prevent running `v`'s destructor so we are in complete control
    // of the allocation.
    let mut v = mem::ManuallyDrop::new(v);

    // Pull out the various important pieces of information about `v`
    let p = v.as_mut_ptr();
    let len = v.len();
    let cap = v.capacity();

    unsafe {
        // Overwrite memory
        for i in 0..len {
            *p.add(i) += 1;
        }

        // Put everything back together into a Vec
        let rebuilt = Vec::from_raw_parts(p, len, cap);
        assert!(kani::forall(|i: usize| (i < len) ==> (rebuilt[i] == original_v[i]+1)));
    }
}
```

The same principle applies if we want to use the existential quantifier.

```rust
use std::ptr;
use std::mem;

#[kani::proof]
fn main() {
    let original_v = vec![kani::any::<usize>(); 3];
    let v = original_v.clone();
    kani::assume(kani::forall(|i: usize| (i < v.len()) ==> (v[i] < 5)));

    // Prevent running `v`'s destructor so we are in complete control
    // of the allocation.
    let mut v = mem::ManuallyDrop::new(v);

    // Pull out the various important pieces of information about `v`
    let p = v.as_mut_ptr();
    let len = v.len();
    let cap = v.capacity();

    unsafe {
        // Overwrite memory
        for i in 0..len {
            *p.add(i) += 1;
            if i == 10 {
              *p.add(i) = 0;
            }
        }

        // Put everything back together into a Vec
        let rebuilt = Vec::from_raw_parts(p, len, cap);
        assert!(kani::exists(|i: usize| (i < len) && (rebuilt[i] == 0)));
    }
}
```

The usage of quantifiers should be valid in any part of the Rust code analysed by Kani.

## Detailed Design

<!-- For the implementors or the hackers -->

Kani should have the same support that CBMC has for quantifiers with its SAT backend. For more details, see [Quantifiers](https://github.com/diffblue/cbmc/blob/0a69a64e4481473d62496f9975730d24f194884a/doc/cprover-manual/contracts-quantifiers.md).


## Open questions

<!-- For Developers -->
- **Function Contracts RFC** - CBMC has support for both `exists` and `forall`, but the
  code generation is difficult. The most ergonomic and easy way to implement
  quantifiers on the Rust side is as higher-order functions taking `Fn(T) ->
  bool`, where `T` is some arbitrary type that can be quantified over. This
  interface is familiar to developers, but the code generation is tricky, as
  CBMC level quantifiers only allow certain kinds of expressions. This
  necessitates a rewrite of the `Fn` closure to a compliant expression.


## Future possibilities

<!-- For Developers -->
- CBMC has an SMT backend which allow the use of quantifiers with arbitrary Boolean expressions. Kani must include an option for users to experiment with this backend.

---
