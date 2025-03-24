// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of the loop contracts code generation.
//!

use proc_macro::TokenStream;
use proc_macro_error2::abort_call_site;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::token::AndAnd;
use syn::{BinOp, Block, Expr, ExprBinary, Ident, Stmt, visit_mut::VisitMut};

/*
Transform the loop to support on_entry(expr) : the value of expr before entering the loop
1. For each on_entry(expr) in the loop variant, replace it with a newly generated "memory" variable old_k
2. Add the declaration of i before the loop: let old_k = expr
For example:
#[kani::loop_invariant(on_entry(x+y) = x + y -1)]
while(....)

is transformed into
let old_1 = x + y
#[kani::loop_invariant(old_1 = x + y -1)]
while(....)

Then the loop_invartiant is transformed

*/

/*
Transform the loop to support prev(expr) : the value of expr at the end of the previous iteration
If there is a prev(expr) in the loop_invariant:
1. Firstly, Add an assertion of the loop_quard: the loop must run at least once, otherwise prev(expr) is undefined.
1. For each prev(expr) in the loop variant, replace it with a newly generated "memory" variable prev_k
2. Add the declaration of i before the loop: let mut prev_k = expr
3. Define a mut closure whose body is exactly the loop body, but replace all break and continue statements with return statements
4. Call the closure (the same as run the loop once)
5. Add the assertion for the loop_invariant (not includes the loop_quard): check if the loop_invariant holds after the first iteration.
6. Add the loop
6. Add the assignment statements (exactly the same as the declarations without the "let mut") on the top of the loop body to update the "memory" variables
For example:
#[kani::loop_invariant(prev(x+y) = x + y -1 && ...)]
while(loop_guard)
{
    loop_body
}

is transformed into

assert!(loop_guard);
let mut prev_1 = x + y;
let mut loop_body_closure = || {
    loop_body_replaced //replace breaks/continues in loop_body with returns
};
loop_body_closure();
assert!(prev_1 = x + y -1 && ...);
#[kani::loop_invariant(prev_1  = x + y -1)]
while(loop_guard)
{
    prev_1 = x + y;
    loop_body
}

*/

/// After that:
/// Expand loop contracts macros.
///
/// A while loop of the form
/// ``` rust
///  while guard {
///      body
///  }
/// ```
/// will be annotated as
/// ``` rust
/// #[inline(never)]
/// #[kanitool::fn_marker = "kani_register_loop_contract"]
/// const fn kani_register_loop_contract_id<T, F: FnOnce() -> T>(f: F) -> T {
///     unreachable!()
/// }
///  while kani_register_loop_contract_id(|| -> bool {inv};) && guard {
///      body
///  }
/// ```

struct TransformationResult {
    transformed_expr: Expr,
    declarations_block: Block,
    assignments_block: Block,
}

struct CallReplacer {
    old_name: String,
    replacements: Vec<(Expr, proc_macro2::Ident)>,
    counter: usize,
    var_prefix: String,
}

// This impl replaces any function call of a function name : old_name with a newly generated variable.
impl CallReplacer {
    fn new(old_name: &str, var_prefix: String) -> Self {
        Self {
            old_name: old_name.to_string(),
            replacements: Vec::new(),
            counter: 0,
            var_prefix: var_prefix,
        }
    }

    fn generate_var_name(&mut self) -> proc_macro2::Ident {
        let var_name = format_ident!("{}_{}", self.var_prefix, self.counter);
        self.counter += 1;
        var_name
    }

    //Check if the function name is old_name
    fn should_replace(&self, expr_path: &syn::ExprPath) -> bool {
        let full_path = expr_path
            .path
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");

        full_path == self.old_name
    }
}

impl VisitMut for CallReplacer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Visit nested expressions first
        syn::visit_mut::visit_expr_mut(self, expr);

        if let Expr::Call(call) = expr {
            if let Expr::Path(expr_path) = &*call.func {
                if self.should_replace(expr_path) {
                    let new_var = self.generate_var_name();
                    self.replacements.push((expr.clone(), new_var.clone()));
                    *expr = syn::parse_quote!(#new_var);
                }
            }
        }
    }
}

// The main function to replace the function call with the variables
fn transform_function_calls(
    expr: Expr,
    function_name: &str,
    var_prefix: String,
) -> TransformationResult {
    let mut replacer = CallReplacer::new(function_name, var_prefix);
    let mut transformed_expr = expr;
    replacer.visit_expr_mut(&mut transformed_expr);

    let mut newreplace: Vec<(Expr, Ident)> = Vec::new();
    for (call, var) in replacer.replacements {
        match call {
            Expr::Call(call_expr) => {
                let insideexpr = call_expr.args[0].clone();
                newreplace.push((insideexpr, var.clone()));
            }
            _ => {}
        }
    }

    // Generate declarations block of the newly generated variables (will added before the loop)
    let declarations: Vec<Stmt> = newreplace
        .iter()
        .map(|(call, var)| syn::parse_quote!(let mut #var = #call.clone();))
        .collect();
    let declarations_block: Block = syn::parse_quote!({
        #(#declarations)*
    });

    // Generate declarations block of the newly generated variables (will be added on the loop of the loop body)
    let assignments: Vec<Stmt> = newreplace
        .into_iter()
        .map(|(call, var)| syn::parse_quote!(#var = #call.clone();))
        .collect();
    let assignments_block: Block = syn::parse_quote!({
        #(#assignments)*
    });

    TransformationResult { transformed_expr, declarations_block, assignments_block }
}

struct BreakContinueReplacer;

impl VisitMut for BreakContinueReplacer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Visit nested expressions first
        syn::visit_mut::visit_expr_mut(self, expr);

        // Replace the expression
        *expr = match expr {
            Expr::Break(_) => {
                syn::parse_quote!(return)
            }
            Expr::Continue(_) => {
                syn::parse_quote!(return)
            }
            _ => return,
        };
    }
}

// This function replace the break/continue statements inside a loop body with return statements
fn transform_break_continue(block: &mut Block) {
    let mut replacer = BreakContinueReplacer;
    replacer.visit_block_mut(block);
}

pub fn loop_invariant(attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse the stmt of the loop
    let mut loop_stmt: Stmt = syn::parse(item.clone()).unwrap();

    // name of the loop invariant as closure of the form
    // __kani_loop_invariant_#startline_#startcol_#endline_#endcol
    let mut inv_name: String = "__kani_loop_invariant".to_owned();
    let loop_id = generate_unique_id_from_span(&loop_stmt);
    inv_name.push_str(&loop_id);

    // expr of the loop invariant
    let mut inv_expr: Expr = syn::parse(attr).unwrap();

    // adding old variables
    let mut old_var_prefix: String = "__kani_old_var".to_owned();
    old_var_prefix.push_str(&loop_id);
    let replace_old: TransformationResult =
        transform_function_calls(inv_expr.clone(), "on_entry", old_var_prefix);
    inv_expr = replace_old.transformed_expr.clone();
    let old_decl_stms = replace_old.declarations_block.stmts.clone();

    // adding prev variables
    let mut prev_var_prefix: String = "__kani_prev_var".to_owned();
    prev_var_prefix.push_str(&loop_id);
    let transform_inv: TransformationResult =
        transform_function_calls(inv_expr.clone(), "prev", prev_var_prefix);
    let has_prev = !transform_inv.declarations_block.stmts.is_empty();
    let decl_stms = transform_inv.declarations_block.stmts.clone();
    let mut assign_stms = transform_inv.assignments_block.stmts.clone();
    let (mut loop_body, loop_guard) = match loop_stmt {
        Stmt::Expr(ref mut e, _) => match e {
            Expr::While(ew) => (ew.body.clone(), ew.cond.clone()),
            _ => panic!(),
        },
        _ => panic!(),
    };
    let loop_body_stms = loop_body.stmts.clone();
    assign_stms.extend(loop_body_stms);
    transform_break_continue(&mut loop_body);
    let mut loop_body_closure_name: String = "__kani_loop_body_closure".to_owned();
    loop_body_closure_name.push_str(&loop_id);
    let loop_body_closure = format_ident!("{}", loop_body_closure_name);
    if has_prev {
        inv_expr = transform_inv.transformed_expr.clone();
        match loop_stmt {
            Stmt::Expr(ref mut e, _) => match e {
                Expr::While(ew) => ew.body.stmts = assign_stms.clone(),
                _ => panic!(),
            },
            _ => panic!(),
        };
    }

    // ident of the register function
    let mut register_name: String = "kani_register_loop_contract".to_owned();
    register_name.push_str(&loop_id);
    let register_ident = format_ident!("{}", register_name);

    match loop_stmt {
        Stmt::Expr(ref mut e, _) => match e {
            Expr::While(ref mut ew) => {
                let new_cond: Expr = syn::parse(
                    quote!(
                        #register_ident(&||->bool{#inv_expr}, 0))
                    .into(),
                )
                .unwrap();
                *(ew.cond) = Expr::Binary(ExprBinary {
                    attrs: Vec::new(),
                    left: Box::new(new_cond),
                    op: BinOp::And(AndAnd::default()),
                    right: ew.cond.clone(),
                });
            }
            _ => {
                abort_call_site!("`#[kani::loop_invariant]` is now only supported for while-loops.";
                    note = "for now, loop contracts is only supported for while-loops.";
                )
            }
        },
        _ => abort_call_site!("`#[kani::loop_invariant]` is now only supported for while-loops.";
            note = "for now, loop contracts is only supported for while-loops.";
        ),
    }

    if has_prev {
        quote!(
        {
        #(#old_decl_stms)*
        assert!(#loop_guard);
        #(#decl_stms)*
        let mut #loop_body_closure = ||
        #loop_body;
        #loop_body_closure ();
        assert!(#inv_expr);
        // Dummy function used to force the compiler to capture the environment.
        // We cannot call closures inside constant functions.
        // This function gets replaced by `kani::internal::call_closure`.
        #[inline(never)]
        #[kanitool::fn_marker = "kani_register_loop_contract"]
        const fn #register_ident<F: Fn() -> bool>(_f: &F, _transformed: usize) -> bool {
            true
        }
        #loop_stmt
        })
        .into()
    } else {
        quote!(
        {
        #(#old_decl_stms)*
        // Dummy function used to force the compiler to capture the environment.
        // We cannot call closures inside constant functions.
        // This function gets replaced by `kani::internal::call_closure`.
        #[inline(never)]
        #[kanitool::fn_marker = "kani_register_loop_contract"]
        const fn #register_ident<F: Fn() -> bool>(_f: &F, _transformed: usize) -> bool {
            true
        }
        #loop_stmt
        })
        .into()
    }
}

fn generate_unique_id_from_span(stmt: &Stmt) -> String {
    // Extract the span of the expression
    let span = stmt.span().unwrap();

    // Get the start and end line and column numbers
    let start = span.start();
    let end = span.end();

    // Create a tuple of location information (file path, start line, start column, end line, end column)
    format!("_{:?}_{:?}_{:?}_{:?}", start.line(), start.column(), end.line(), end.column())
}
