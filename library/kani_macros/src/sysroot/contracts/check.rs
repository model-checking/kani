// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic used for generating the code that checks a contract.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::mem;
use syn::{parse_quote, Block, Expr, Local, LocalInit, Pat, PatIdent, ReturnType, Stmt};

use super::{
    helpers::*, shared::build_ensures, ContractConditionsData, ContractConditionsHandler,
    INTERNAL_RESULT_IDENT,
};

const WRAPPER_ARG: &str = "_wrapper_arg";

impl<'a> ContractConditionsHandler<'a> {
    /// Create the body of a check function.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    pub fn make_check_body(&self, mut body_stmts: Vec<Stmt>) -> TokenStream2 {
        let Self { attr_copy, .. } = self;
        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                quote!({
                    kani::assume(#attr);
                    #(#body_stmts)*
                })
            }
            ContractConditionsData::Ensures { attr } => {
                let (remembers, ensures_clause) = build_ensures(attr);

                // The code that enforces the postconditions and cleans up the shallow
                // argument copies (with `mem::forget`).
                let exec_postconditions = quote!(
                    kani::assert(#ensures_clause, stringify!(#attr_copy));
                );

                let return_expr = body_stmts.pop();
                quote!({
                    #remembers
                    #(#body_stmts)*
                    #exec_postconditions
                    #return_expr
                })
            }
            ContractConditionsData::Modifies { attr } => {
                let wrapper_arg_ident = Ident::new(WRAPPER_ARG, Span::call_site());
                let wrapper_tuple = body_stmts.iter_mut().find_map(|stmt| {
                    if let Stmt::Local(Local {
                        pat: Pat::Ident(PatIdent { ident, .. }),
                        init: Some(LocalInit { expr, .. }),
                        ..
                    }) = stmt
                    {
                        (ident == &wrapper_arg_ident).then_some(expr.as_mut())
                    } else {
                        None
                    }
                });
                if let Some(Expr::Tuple(values)) = wrapper_tuple {
                    values.elems.extend(attr.iter().map(|attr| {
                        let expr: Expr = parse_quote!(#attr
                        as *const _);
                        expr
                    }));
                } else {
                    unreachable!("Expected tuple but found `{wrapper_tuple:?}`")
                }
                quote!({#(#body_stmts)*})
            }
        }
    }

    /// Initialize the list of statements for the check closure body.
    fn initial_check_stmts(&self) -> Vec<syn::Stmt> {
        let modifies_ident = Ident::new(&self.modify_name, Span::call_site());
        let wrapper_arg_ident = Ident::new(WRAPPER_ARG, Span::call_site());
        let return_type = return_type_to_type(&self.annotated_fn.sig.output);
        let inputs = closure_params(&self.annotated_fn.sig.inputs);
        let modifies_closure = self.modifies_closure(
            inputs,
            &self.annotated_fn.sig.output,
            &self.annotated_fn.block,
            true,
        );
        let args = closure_args(&self.annotated_fn.sig.inputs);
        let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
        parse_quote!(
            let #wrapper_arg_ident = ();
            #modifies_closure
            let #result : #return_type = #modifies_ident(#(#args,)* #wrapper_arg_ident);
            #result
        )
    }

    /// Generate a token stream that represents the check closure.
    ///
    /// See [`Self::make_check_body`] for the most interesting parts of this
    /// function.
    pub fn check_closure(&self) -> TokenStream2 {
        let check_ident = Ident::new(&self.check_name, Span::call_site());
        let sig = &self.annotated_fn.sig;
        let inputs = closure_params(&sig.inputs);
        let output = &sig.output;
        let body_stmts = self.initial_check_stmts();
        let body = self.make_check_body(body_stmts);

        quote!(
            #[kanitool::is_contract_generated(check)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #check_ident = |#(#inputs),*| #output #body;
        )
    }

    /// Expand the check body.
    ///
    /// First find the modifies body and expand that. Then expand the rest of the body.
    pub fn expand_check(&self, closure: &mut Stmt) {
        let body = closure_body(closure);
        self.expand_modifies(find_contract_closure(&mut body.block.stmts, "wrapper"));
        *body = syn::parse2(self.make_check_body(mem::take(&mut body.block.stmts))).unwrap();
    }

    /// Emit a modifies wrapper. The first time, we augment the list of inputs to track modifies.
    pub fn modifies_closure<T: ToTokens, I: IntoIterator<Item = T>>(
        &self,
        inputs: I,
        output: &ReturnType,
        body: &Block,
        include_modifies: bool,
    ) -> TokenStream2 {
        // Filter receiver
        let wrapper_ident = include_modifies.then_some(Ident::new(WRAPPER_ARG, Span::call_site()));
        let wrapper_it = wrapper_ident.iter();
        let modifies_ident = Ident::new(&self.modify_name, Span::call_site());
        let inputs = inputs.into_iter();
        quote!(
            #[kanitool::is_contract_generated(wrapper)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #modifies_ident = |#(#inputs,)* #(#wrapper_it: _,)*| #output #body;
        )
    }

    /// Expand the modifies closure if we are handling a modifies attribute. Otherwise, no-op.
    pub fn expand_modifies(&self, closure_stmt: &mut Stmt) {
        if matches!(&self.condition_type, ContractConditionsData::Modifies { .. }) {
            let Stmt::Local(Local { init: Some(LocalInit { expr, .. }), .. }) = closure_stmt else {
                unreachable!()
            };
            let Expr::Closure(closure) = expr.as_ref() else { unreachable!() };
            let Expr::Block(body) = closure.body.as_ref() else { unreachable!() };
            let stream =
                self.modifies_closure(&closure.inputs, &closure.output, &body.block, false);
            *closure_stmt = syn::parse2(stream).unwrap();
        }
    }
}
