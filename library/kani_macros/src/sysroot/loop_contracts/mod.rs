// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of the loop contracts code generation.
//!

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro_error::abort_call_site;
use quote::quote;
use syn::{Expr, Stmt};

pub fn loop_invariant(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut loop_stmt: Stmt = syn::parse(item.clone()).unwrap();

    // Annotate a place holder function call at the end of the loop.
    match loop_stmt {
        Stmt::Expr(ref mut e, _) => match e {
            Expr::While(ref mut ew) => {
                // A while loop of the form
                // ``` rust
                //  while guard {
                //      body
                //  }
                // ```
                // is annotated as
                // ``` rust
                //  while guard{
                //      body
                //      kani::kani_loop_invariant_begin_marker();
                //      let __kani_loop_invariant: bool = inv;
                //      kani::kani_loop_invariant_end_marker();
                //  }
                // ```
                let mut to_parse = quote!(
                     let __kani_loop_invariant: bool = );
                to_parse.extend(TokenStream2::from(attr.clone()));
                to_parse.extend(quote!(;));
                let inv_assign_stmt: Stmt = syn::parse(to_parse.into()).unwrap();

                //      kani::kani_loop_invariant_begin_marker();
                let inv_begin_stmt: Stmt = syn::parse(
                    quote!(
                    kani::kani_loop_invariant_begin_marker();)
                    .into(),
                )
                .unwrap();

                //      kani::kani_loop_invariant_end_marker();
                let inv_end_stmt: Stmt = syn::parse(
                    quote!(
                kani::kani_loop_invariant_end_marker();)
                    .into(),
                )
                .unwrap();
                ew.body.stmts.push(inv_begin_stmt);
                ew.body.stmts.push(inv_assign_stmt);
                ew.body.stmts.push(inv_end_stmt);
            }
            _ => (),
        },
        _ => abort_call_site!("`#[kani::loop_invariant]` is not only supported for while-loops.";
            note = "for now, loop contracts is only supported for while-loops.";
        ),
    }

    quote!(;
        #loop_stmt;)
    .into()
}
