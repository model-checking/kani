# Pure Expression Inliner

## Overview

The pure expression inliner (`inline_as_pure_expr`) inlines function calls
within expression trees as side-effect-free expressions. Unlike the existing
`inline_function_calls_in_expr` which wraps inlined bodies in CBMC
`StatementExpression` nodes, this produces pure expression trees.

## Soundness Implications

**Checked arithmetic in quantifier bodies**: When flattening `StatementExpression`
nodes (e.g., from checked division or remainder), the pure inliner drops the
`Assert` and `Assume` statements that check for overflow and division by zero.

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
