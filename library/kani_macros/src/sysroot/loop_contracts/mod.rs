// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of the loop contracts code generation.
//!

use proc_macro::TokenStream;
use proc_macro_error2::abort_call_site;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::token::AndAnd;
use syn::{BinOp, Expr, ExprBinary, Stmt};

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
pub fn loop_invariant(attr: TokenStream, item: TokenStream) -> TokenStream {
    // parse the stmt of the loop
    let mut loop_stmt: Stmt = syn::parse(item.clone()).unwrap();

    // name of the loop invariant as closure of the form
    // __kani_loop_invariant_#startline_#startcol_#endline_#endcol
    let mut inv_name: String = "__kani_loop_invariant".to_owned();
    let loop_id = generate_unique_id_from_span(&loop_stmt);
    inv_name.push_str(&loop_id);

    // expr of the loop invariant
    let inv_expr: Expr = syn::parse(attr).unwrap();

    // ident of the register function
    let mut register_name: String = "kani_register_loop_contract".to_owned();
    register_name.push_str(&loop_id);
    let register_ident = format_ident!("{}", register_name);

    match loop_stmt {
        Stmt::Expr(ref mut e, _) => match e {
            Expr::While(ref mut ew) => {
                let new_cond: Expr = syn::parse(
                    quote!(
                        #register_ident(||->bool{#inv_expr}))
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
    quote!(
        {
        // Dummy function used to force the compiler to capture the environment.
        // We cannot call closures inside constant functions.
        // This function gets replaced by `kani::internal::call_closure`.
        #[inline(never)]
        #[kanitool::fn_marker = "kani_register_loop_contract"]
        const fn #register_ident<T, F: FnOnce() -> T>(f: F) -> T {
            unreachable!()
        }
        #loop_stmt})
    .into()
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
