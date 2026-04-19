- **Feature Name:** Quantifiers
- **Feature Request Issue:** [#2546](https://github.com/model-checking/kani/issues/2546) and [#836](https://github.com/model-checking/kani/issues/836)
- **RFC PR:** [#](https://github.com/model-checking/kani/pull/)
- **Status:** Under Review
- **Version:** 1.0

-------------------

## Summary

Quantifiers are logical operators that allow users to express that a property or condition applies to some or all objects within a given domain.

## User Impact

There are two primary quantifiers: the existential quantifier (∃) and the universal quantifier (∀).

1. The existential quantifier (∃): represents the statement "there exists." We use to express that there is at least one object in the domain that satisfies a given condition. For example, "∃x P(x)" means "there exists a value x such that P(x) is true."

2. The universal quantifier (∀): represents the statement "for all" or "for every." We use it to express that a given condition is true for every object in the domain. For example, "∀x P(x)" means "for every value x, P(x) is true."

Rather than exhaustively listing all elements in a domain, quantifiers enable users to make statements about the entire domain at once. This compact representation is crucial when dealing with large or unbounded inputs. Quantifiers also facilitate abstraction and generalization of properties. Instead of specifying properties for specific instances, quantified properties can capture general patterns and behaviors that hold across different objects in a domain. Additionally, by replacing loops in the specification with quantifiers, Kani can encode the properties more efficiently within the specified bounds, making the verification process more manageable and computationally feasible.

This new feature doesn't introduce any breaking changes to users. It will only allow them to write properties using the existential (∃) and universal (∀) quantifiers.

## User Experience

The syntax of existential (i.e., `kani::exists`) and universal (i.e., `kani::forall`) quantifiers are:

```rust
kani::exists(|<var>: <type> [in (<range-expr>)] | <boolean-expression>)
kani::forall(|<var>: <type> [in (<range-expr>)] | <boolean-expression>)
```

If `<range-expr>` is not provided, we assume `<var>` can range over all possible values of the given `<type>` (i.e., syntactic sugar for full range `|<var>: <type> as .. |`). CBMC's SAT backend only supports bounded quantification under **constant** lower and upper bounds (for more details, see the [documentation for quantifiers in CBMC](https://diffblue.github.io/cbmc/contracts-quantifiers.html)). The SMT backend, on the other hand, supports arbitrary Boolean expressions. In any case, `<boolean-expression>` should not have side effects, as the purpose of quantifiers is to assert a condition over a domain of objects without altering the state.

Consider the following example adapted from the documentation for the [from_raw_parts](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.from_raw_parts) function:

```rust
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

Given the `v` vector has non-deterministic values, there are potential arithmetic overflows that might happen in the for loop. So we need to constrain all values of the array. We may also want to check all values of `rebuilt` after the operation. Without quantifiers, we might be tempted to use loops as follows:

```rust
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

This, however, might unnecessary increase the complexity of the verification process. We can achieve the same effect using quantifiers as shown below.

```rust
use std::mem;

#[kani::proof]
fn main() {
    let original_v = vec![kani::any::<u32>(); 3];
    let v = original_v.clone();
    let v_len = v.len();
    let v_ptr = v.as_ptr();
    let original_v_ptr = original_v.as_ptr();
    unsafe {
        kani::assume(
            kani::forall!(|i in (0,v_len) | *v_ptr.wrapping_byte_offset(4*i as isize) < 5),
        );
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
            if i == 1 {
                *p.add(i) = 0;
            }
        }

        // Put everything back together into a Vec
        let rebuilt = Vec::from_raw_parts(p, len, cap);
        let rebuilt_ptr = v.as_ptr();
        assert!(
            kani::exists!(| i in (0, len) | *rebuilt_ptr.wrapping_byte_offset(4*i as isize) == original_v_ptr.wrapping_byte_offset(4*i as isize) + 1)
        );
    }
}
```

The same principle applies if we want to use the existential quantifier.

```rust
use std::mem;

#[kani::proof]
fn main() {
    let original_v = vec![kani::any::<u32>(); 3];
    let v = original_v.clone();
    let v_len = v.len();
    let v_ptr = v.as_ptr();
    unsafe {
        kani::assume(
            kani::forall!(|i in (0,v_len) | *v_ptr.wrapping_byte_offset(4*i as isize) < 5),
        );
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
            if i == 1 {
                *p.add(i) = 0;
            }
        }

        // Put everything back together into a Vec
        let rebuilt = Vec::from_raw_parts(p, len, cap);
        let rebuilt_ptr = v.as_ptr();
        assert!(
            kani::exists!(| i in (0, len) | *rebuilt_ptr.wrapping_byte_offset(4*i as isize) == 0)
        );
    }
}
```

The usage of quantifiers should be valid in any part of the Rust code analysed by Kani.

## Detailed Design

<!-- For the implementors or the hackers -->

Kani should have the same support that CBMC has for quantifiers. For more details, see [Quantifiers](https://github.com/diffblue/cbmc/blob/0a69a64e4481473d62496f9975730d24f194884a/doc/cprover-manual/contracts-quantifiers.md).

### CBMC constraint: side-effect-free expressions

CBMC's quantifiers only support single expressions without function calls or side
effects. However, even simple Rust expressions like `i + 1` or `i % 2` compile to
checked arithmetic operations (`OverflowResultPlus`, `checked_rem`) that produce
`StatementExpression` nodes in the GOTO program — which CBMC rejects as side effects.

### Pure expression codegen

To solve this, Kani generates **pure expression trees** for quantifier bodies:

1. **Closure body extraction**: `build_quantifier_predicate` in `hooks.rs` extracts
   the closure's codegen'd body from the symbol table, resolves intermediate variable
   assignments into a single expression, and substitutes the closure parameter with
   the quantified variable.

2. **Pure expression inlining**: Before symbol substitution, `inline_as_pure_expr_toplevel`
   (from `goto_ctx.rs`) flattens all `StatementExpression` nodes by:
   - Collecting `Decl` assignments from the statement block
   - Resolving intermediate variables via iterative `substitute_symbol`
   - Extracting the final expression value

3. **Overflow simplification**: The `Member` handler in the inliner simplifies
   overflow-checked arithmetic patterns:
   - `Member(Struct([result, overflowed, padding]), "0")` → extracts `result` directly
   - `Member(OverflowResultPlus(a, b), "result")` → `Plus(a, b)`
   - Same for `OverflowResultMinus` → `Minus` and `OverflowResultMult` → `Mult`

   This drops the overflow check, which is acceptable inside quantifier bodies
   because CBMC evaluates them symbolically.

4. **Function call inlining**: Remaining function calls (e.g., `checked_rem` for `%`)
   are inlined by looking up the function body in the symbol table, resolving its
   return expression, and substituting parameters — producing a pure expression.

5. **Pointer arithmetic intrinsic lowering**: Wrapping pointer arithmetic functions
   (`wrapping_byte_offset`, `wrapping_add`) are compiler intrinsics with no GOTO body
   to inline. The inliner recognizes these by name and lowers them directly to CBMC
   `Plus` expressions on pointers. Non-wrapping variants (`offset`, `add`) are not
   supported because they trigger CBMC bounds checks inside quantifier bodies. Example:
   ```rust
   kani::forall!(|i in (0, len)| unsafe { *ptr.wrapping_byte_offset(i as isize) == 0 })
   ```

### Typed quantifier variables

The `forall!` and `exists!` macros support an optional type annotation:

```rust
kani::forall!(|d: u64 in (1, n)| x % d == 0)
```

The type is captured as `$t:tt` (not `$t:ty`, since `ty` fragments cannot be
followed by `in` in macro rules). The internal `kani_forall<T, F>` function is
already generic over `T`, so the macro simply passes the typed bounds and closure.

### Soundness notes

- **Checked arithmetic dropped**: Overflow checks inside quantifier bodies are
  silently removed. Division by zero inside quantifier bodies is also not detected.
  Users must ensure their predicates don't overflow or divide by zero.
- **Recursive functions**: Recursive function calls in quantifier bodies are detected
  via a `visited` set and the original expression is returned unchanged (CBMC will
  reject it as a side effect).

## Open questions

<!-- For Developers -->
- **Function Contracts RFC** - CBMC has support for both `exists` and `forall`, but the
  code generation is difficult. The most ergonomic and easy way to implement
  quantifiers on the Rust side is as higher-order functions taking `Fn(T) ->
  bool`, where `T` is some arbitrary type that can be quantified over. This
  interface is familiar to developers, but the code generation is tricky, as
  CBMC level quantifiers only allow certain kinds of expressions. This
  necessitates a rewrite of the `Fn` closure to a compliant expression.
  - Which kind of expressions should be accepted as a "compliant expression"?

## Future possibilities

<!-- For Developers -->
- CBMC has an SMT backend which allows the use of quantifiers with arbitrary Boolean expressions. Kani must include an option for users to experiment with this backend.

---
