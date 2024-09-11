// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of the loop contracts code generation.
//!

use proc_macro::TokenStream;
use proc_macro_error::abort_call_site;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Expr, Stmt};

fn generate_unique_id_from_span(stmt: &Stmt) -> String {
    // Extract the span of the expression
    let span = stmt.span().unwrap();

    // Get the start and end line and column numbers
    let start = span.start();
    let end = span.end();

    // Create a tuple of location information (file path, start line, start column, end line, end column)
    format!("_{:?}_{:?}_{:?}_{:?}", start.line(), start.column(), end.line(), end.column())
}

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
/// let __kani_loop_invariant_id = || -> bool {inv};
/// // The register function call with the actual invariant.
/// kani_register_loop_contract_id(__kani_loop_invariant_id);
///  while guard {
///      body
///      // Call to the register function with a dummy argument
///      // for the sake of bypassing borrow checks.
///      kani_register_loop_contract_id(||->bool{true});
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
    let inv_ident = format_ident!("{}", inv_name);

    // expr of the loop invariant
    let inv_expr: Expr = syn::parse(attr).unwrap();

    // ident of the register function
    let mut register_name: String = "kani_register_loop_contract".to_owned();
    register_name.push_str(&loop_id);
    let register_ident = format_ident!("{}", register_name);

    match loop_stmt {
        Stmt::Expr(ref mut e, _) => match e {
            Expr::While(ref mut ew) => {
                //      kani_register_loop_contract(#inv_ident);
                let inv_end_stmt: Stmt = syn::parse(
                    quote!(
                        #register_ident(||->bool{true});)
                    .into(),
                )
                .unwrap();
                ew.body.stmts.push(inv_end_stmt);
            }
            _ => (),
        },
        _ => abort_call_site!("`#[kani::loop_invariant]` is not only supported for while-loops.";
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
        let mut #inv_ident = || -> bool {#inv_expr};
        #register_ident(#inv_ident);
        #loop_stmt})
    .into()
}
