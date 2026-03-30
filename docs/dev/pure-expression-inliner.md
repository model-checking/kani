# Pure Expression Inliner

## Overview

The pure expression inliner (`inline_as_pure_expr`) is a function call inlining
mechanism that produces side-effect-free expression trees. Unlike the original
`inline_function_calls_in_expr` which wraps inlined bodies in CBMC
`StatementExpression` nodes, this produces expressions using only pure
constructs: `BinOp`, `UnOp`, `If` (ternary), `Typecast`, etc.

## Motivation

CBMC's quantifier expressions (`forall`, `exists`) reject side effects in their
bodies. The original inliner produced `StatementExpression` nodes which CBMC
treats as side effects, causing invariant violations. The pure inliner eliminates
this by producing expression trees that CBMC can process directly.

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
1. Collects all assignments: `{local1 → expr1(param1), local2 → expr2(local1, param2)}`
2. Finds the return symbol: `local2`
3. Resolves intermediates: `local2` → `expr2(local1, param2)` → `expr2(expr1(param1), param2)`
4. Substitutes parameters: `expr2(expr1(arg1), arg2)`
5. Flattens `StatementExpression` nodes (e.g., checked arithmetic → just the operation)
6. Recursively inlines any remaining function calls

## Soundness Implications

**Checked arithmetic in quantifier bodies**: When flattening `StatementExpression`
nodes (e.g., from checked division or remainder), the pure inliner drops the
`Assert` and `Assume` statements that check for overflow and division by zero.
This means:

- **Division by zero** inside a quantifier body will NOT be detected. For example,
  `forall!(|i in (0, 10)| arr[i] / x == 0)` where `x` could be zero will not
  produce a division-by-zero check.
- **Arithmetic overflow** inside a quantifier body will NOT be detected.

This is a known trade-off: CBMC requires pure expressions in quantifier bodies,
and runtime checks are inherently side effects. Users should ensure that
arithmetic operations in quantifier predicates cannot overflow or divide by zero.

**Future improvement**: The dropped assertions could be hoisted outside the
quantifier as preconditions, preserving soundness while keeping the quantifier
body pure.

## Limitations

- **No control flow**: Functions with `if`/`else` or `match` that produce
  multiple assignments to the return variable are not fully supported. The
  inliner takes the last assignment, which may not be correct for all paths.
- **No loops**: Functions containing loops cannot be inlined as pure expressions.
- **No recursion**: Recursive functions are detected and cause a panic.
- **Checked arithmetic**: Overflow/division-by-zero checks (`Assert` + `Assume`
  statements) are dropped when flattening `StatementExpression` nodes. This
  means the pure expression doesn't include these runtime checks.

## Files

- `cprover_bindings/src/goto_program/expr.rs` — `Expr::substitute_symbol()`
- `kani-compiler/src/codegen_cprover_gotoc/context/goto_ctx.rs` — `inline_as_pure_expr()`,
  `inline_call_as_pure_expr()`, `collect_assignments_from_stmt()`,
  `find_return_symbol_in_stmt()`, `resolve_intermediates_iterative()`
