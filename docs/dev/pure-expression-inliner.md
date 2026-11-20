# Pure Expression Inliner

## Overview

The pure expression inliner (`inline_as_pure_expr`) inlines function calls
within expression trees as side-effect-free expressions. Unlike the existing
`inline_function_calls_in_expr` which wraps inlined bodies in CBMC
`StatementExpression` nodes, this produces pure expression trees.

## How It Works

For a function call `f(arg1, arg2)` where `f` is defined as:
```c
ret_type f(param1, param2) {
    local1 = expr1(param1);
    local2 = expr2(local1, param2);
    return local2;
}
```

The pure inliner:
1. **Collects assignments**: `{local1 → expr1(param1), local2 → expr2(local1, param2)}`
2. **Finds the return symbol**: `local2`
3. **Resolves intermediates**: `local2` → `expr2(local1, param2)` → `expr2(expr1(param1), param2)`
4. **Flattens StatementExpressions**: e.g., `({ assert(2!=0); assume(2!=0); i%2 })` → `i%2`
5. **Substitutes parameters**: `expr2(expr1(arg1), arg2)`
6. **Recursively inlines** any remaining function calls in the result

The resolution step (3) uses `Expr::substitute_symbol` iteratively until a
fixed point is reached. Change detection uses the `(Expr, bool)` return from
`substitute_symbol` — no string comparison needed.

## Soundness Implications

### Why this transformation preserves Rust semantics

The pure inliner operates on GOTO-program expressions **after** Kani's MIR-to-GOTO
codegen. At this stage, Rust's high-level semantics (ownership, borrowing, lifetimes)
have already been lowered to explicit memory operations. The transformation preserves
the return value semantics of functions under the following reasoning:

**Straight-line code (no aliasing).** For a function whose body is a sequence of
assignments `x1 = e1; x2 = e2(x1); return x2`, the inliner substitutes `x2` →
`e2(x1)` → `e2(e1)`, producing the same value as executing the assignments
sequentially. This is the standard algebraic substitution used in SSA-based
optimizations and is semantics-preserving when each local is assigned exactly once
(SSA form). Rust's MIR is in SSA form for local variables, and GOTO codegen
preserves this property for the common case.

**Aliasing and memory references.** The inliner substitutes *symbol names* in
expressions, not memory locations. Expressions involving pointer dereferences
(`*ptr`), struct field accesses (`.field`), and array indexing (`arr[i]`) are
preserved structurally — the inliner does not evaluate them, it only replaces
the symbol that names the pointer/array/struct. For example:

```
local1 = *ptr;          // read through pointer
local2 = local1 + 1;
return local2;
```

becomes `*ptr + 1` — the dereference is preserved in the expression tree and
CBMC evaluates it at verification time with full alias analysis. The inliner
does NOT assume anything about what `*ptr` points to; it merely substitutes
the symbol `local1` with the expression `*ptr`.

**When the transformation is NOT safe.** The inliner is unsound when:

1. **A local is assigned multiple times** (e.g., in different branches of an
   `if`/`else`). The inliner takes the last assignment, which may not correspond
   to the executed branch. A `tracing::debug!` diagnostic is emitted. This case
   does not arise for quantifier closures in practice because they are simple
   single-expression bodies.

2. **Side effects between assignments matter.** If `local1 = f()` has a side
   effect (e.g., writing to a global) that `local2 = g(local1)` depends on,
   substituting `g(f())` changes the evaluation order. However, in the GOTO
   program, side effects are explicit statements — the inliner only substitutes
   within the expression tree, not across statements with side effects. The
   `StatementExpression` flattening (which drops `Assert`/`Assume`) is the one
   place where side effects are lost (see below).

3. **Mutable aliasing across assignments.** If two locals alias the same memory
   and one is written between the assignments:
   ```
   local1 = *ptr;
   *ptr = 42;        // modifies what ptr points to
   local2 = *ptr;    // now different from local1
   return local2;
   ```
   Substituting `local2` → `*ptr` is correct (it reads the current value), but
   substituting `local1` → `*ptr` in an expression that uses both would be wrong
   because `local1` captured the old value. The inliner handles this correctly
   because it substitutes from the return expression backward — `local2` is
   replaced by its RHS (`*ptr`), and `local1` is only substituted if it appears
   in `local2`'s RHS, which it doesn't in this case.

**Scope of applicability.** This inliner is designed specifically for quantifier
predicate closures, which are typically pure, single-expression functions with no
mutable aliasing, no loops, and no control flow. For these functions, the
substitution is equivalent to β-reduction in lambda calculus and is
semantics-preserving.

### Checked arithmetic

When flattening `StatementExpression` nodes (e.g., from checked division or
remainder), the pure inliner drops the `Assert` and `Assume` statements that
check for overflow and division by zero.

- **Division by zero** inside a quantifier body will NOT be detected.
- **Arithmetic overflow** inside a quantifier body will NOT be detected.

**Future improvement**: The dropped assertions could be hoisted outside the
quantifier as preconditions, preserving soundness while keeping the body pure.

## Limitations

- **No control flow**: Functions with `if`/`else` or `match` that produce
  multiple assignments to the return variable are not fully supported. The
  inliner takes the last assignment and emits a `tracing::debug!` diagnostic.
- **No loops**: Functions containing loops cannot be inlined as pure expressions.
- **No recursion**: Recursive functions are detected and the original expression
  is returned unchanged (with a `tracing::warn!` diagnostic). No ICE.
- **StatementExpression in substitute_symbol**: `Expr::substitute_symbol` does
  NOT recurse into `StatementExpression` nodes. These must be flattened via
  `inline_as_pure_expr` before substitution.

## API

```rust
// Public entry point — manages the visited set internally
pub fn inline_as_pure_expr_toplevel(&self, expr: &Expr) -> Expr;

// Expr method — returns (new_expr, changed) for reliable change detection
pub fn substitute_symbol(self, old_id: &InternedString, replacement: &Expr) -> (Expr, bool);
```

## Files

- `cprover_bindings/src/goto_program/expr.rs` — `Expr::substitute_symbol()`
- `kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs` — `inline_as_pure_expr()`,
  `inline_as_pure_expr_toplevel()`, `inline_call_as_pure_expr()`,
  `collect_assignments_from_stmt()`, `find_return_symbol_in_stmt()`,
  `resolve_intermediates_iterative()`
