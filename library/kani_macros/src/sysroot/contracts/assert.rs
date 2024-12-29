// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic used for generating the code that generates contract preconditions and postconditions as assertions.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use std::mem;
use syn::{Stmt, parse_quote};

use super::{
    ContractConditionsData, ContractConditionsHandler, ContractMode, INTERNAL_RESULT_IDENT,
    helpers::*,
    shared::{build_ensures, split_for_remembers},
};

impl<'a> ContractConditionsHandler<'a> {
    /// Generate a token stream that represents the assert closure.
    ///
    /// See [`Self::make_assert_body`] for the most interesting parts of this
    /// function.
    pub fn assert_closure(&self) -> TokenStream2 {
        let assert_ident = Ident::new(&self.assert_name, Span::call_site());
        let sig = &self.annotated_fn.sig;
        let output = &sig.output;
        let body_stmts = self.initial_assert_stmts();
        let body = self.make_assert_body(body_stmts);

        quote!(
            #[kanitool::is_contract_generated(assert)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #assert_ident = || #output #body;
        )
    }

    /// Expand the assert closure body.
    pub fn expand_assert(&self, closure: &mut Stmt) {
        let body = closure_body(closure);
        *body = syn::parse2(self.make_assert_body(mem::take(&mut body.block.stmts))).unwrap();
    }

    /// Initialize the list of statements for the assert closure body.
    fn initial_assert_stmts(&self) -> Vec<syn::Stmt> {
        let return_type = return_type_to_type(&self.annotated_fn.sig.output);
        let stmts = &self.annotated_fn.block.stmts;
        let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
        parse_quote! {
            let #result : #return_type = {#(#stmts)*};
            #result
        }
    }

    /// Create the body of an assert closure.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    fn make_assert_body(&self, mut body_stmts: Vec<Stmt>) -> TokenStream2 {
        let Self { attr_copy, .. } = self;
        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                quote!({
                    kani::assert(#attr, stringify!(#attr_copy));
                    #(#body_stmts)*
                })
            }
            ContractConditionsData::Ensures { attr } => {
                let (remembers, ensures_clause) = build_ensures(attr);

                let exec_postconditions = quote!(
                    kani::assert(#ensures_clause, stringify!(#attr_copy));
                );

                let return_expr = body_stmts.pop();

                let (asserts, rest_of_body) =
                    split_for_remembers(&body_stmts[..], ContractMode::Assert);

                quote!({
                    #(#asserts)*
                    #remembers
                    #(#rest_of_body)*
                    #exec_postconditions
                    #return_expr
                })
            }
            ContractConditionsData::Modifies { .. } => {
                quote!({#(#body_stmts)*})
            }
        }
    }
}
