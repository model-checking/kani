// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic used for generating the code that checks a contract.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use std::mem;
use syn::{Block, Expr, FnArg, Local, LocalInit, Pat, PatIdent, ReturnType, Stmt, parse_quote};

use super::{
    ContractConditionsData, ContractConditionsHandler, ContractMode, INTERNAL_RESULT_IDENT,
    helpers::*,
    shared::{build_ensures, split_for_remembers},
};

const WRAPPER_ARG: &str = "_wrapper_arg";

impl<'a> ContractConditionsHandler<'a> {
    /// Create the body of a check function.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    pub fn make_check_body(&self, mut body_stmts: Vec<Stmt>) -> TokenStream2 {
        if let Some(error) = duplicate_panic_clause(&self.condition_type, &body_stmts) {
            return error;
        }
        let Self { attr_copy, .. } = self;
        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                let assume_bracketed = bracket_clause_stmt(quote!(kani::assume(#attr);));
                quote!({
                    #assume_bracketed
                    #(#body_stmts)*
                })
            }
            ContractConditionsData::PanicsOnlyIf { attr } => {
                // Record on entry, after the preconditions are assumed, whether a panic is
                // permitted, so that Kani checks every panic of the body against it. The
                // permission is restored when the function returns.
                let cond = bracket_clause_expr(quote!(#attr));
                let return_expr = body_stmts.pop();
                let (assumes, rest_of_body) =
                    split_for_remembers(&body_stmts[..], ContractMode::SimpleCheck);
                quote!({
                    #(#assumes)*
                    let __kani_panics_only_if_prev = kani::internal::set_panics_only_if(#cond);
                    #(#rest_of_body)*
                    kani::internal::restore_panic_permission(__kani_panics_only_if_prev);
                    #return_expr
                })
            }
            ContractConditionsData::PanicsIf { attr } => {
                // As for `panics_only_if`, and additionally check on return that the condition
                // did not hold: if it held, the function must have panicked.
                let cond = bracket_clause_expr(quote!(#attr));
                let return_expr = body_stmts.pop();
                let (assumes, rest_of_body) =
                    split_for_remembers(&body_stmts[..], ContractMode::SimpleCheck);
                quote!({
                    #(#assumes)*
                    let __kani_panics_if_cond: bool = #cond;
                    let __kani_panics_if_prev =
                        kani::internal::set_panics_if(__kani_panics_if_cond);
                    #(#rest_of_body)*
                    kani::assert(
                        !__kani_panics_if_cond,
                        concat!("returned although `panics_if(", stringify!(#attr_copy), ")` held"),
                    );
                    kani::internal::restore_panic_permission(__kani_panics_if_prev);
                    #return_expr
                })
            }
            ContractConditionsData::Ensures { attr } => {
                let (remembers, ensures_clause) = build_ensures(attr);

                // The code that enforces the postconditions and cleans up the shallow
                // argument copies (with `mem::forget`).
                let ensures_bracketed = bracket_clause_expr(quote!(#ensures_clause));
                let exec_postconditions = quote!(
                    kani::assert(#ensures_bracketed, stringify!(#attr_copy));
                );

                let return_expr = body_stmts.pop();

                let (assumes, rest_of_body) =
                    split_for_remembers(&body_stmts[..], ContractMode::SimpleCheck);

                quote!({
                    #(#assumes)*
                    #remembers
                    #(#rest_of_body)*
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
                        let bracketed = bracket_clause_expr(quote!(#attr as *const _));
                        let expr: Expr = parse_quote!(#bracketed);
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
        let mut_recv = self.has_mutable_receiver().then(|| quote!(core::ptr::addr_of!(self),));
        let redefs_mut_only = self.arg_redefinitions(true);
        let modifies_closure = self.modifies_closure(
            &self.annotated_fn.sig.output,
            &self.annotated_fn.block,
            redefs_mut_only,
        );
        let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
        parse_quote!(
            let #wrapper_arg_ident = (#mut_recv);
            #modifies_closure
            let #result : #return_type = #modifies_ident(#wrapper_arg_ident);
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
        let output = &sig.output;
        let body_stmts = self.initial_check_stmts();
        let body = self.make_check_body(body_stmts);

        quote!(
            #[kanitool::is_contract_generated(check)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #check_ident = kani_force_fn_once(|| #output #body);
        )
    }

    /// Expand the check body.
    ///
    /// First find the modifies body and expand that. Then expand the rest of the body.
    pub fn expand_check(&self, closure: &mut Stmt) {
        let body = closure_body(closure);
        self.expand_modifies(
            find_contract_closure(&mut body.block.stmts, "wrapper")
                .expect("Internal Failure: Expected to find `wrapper` closure, but found none"),
        );
        *body = syn::parse2(self.make_check_body(mem::take(&mut body.block.stmts))).unwrap();
    }

    /// Emit a modifies wrapper. It's only argument is the list of addresses that may be modified.
    pub fn modifies_closure(
        &self,
        output: &ReturnType,
        body: &Block,
        redefs: TokenStream2,
    ) -> TokenStream2 {
        // Filter receiver
        let wrapper_ident = Ident::new(WRAPPER_ARG, Span::call_site());
        let modifies_ident = Ident::new(&self.modify_name, Span::call_site());
        let stmts = &body.stmts;
        quote!(
            #[kanitool::is_contract_generated(wrapper)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #modifies_ident = kani_force_fn_once_with_args(|#wrapper_ident: _| #output {
                #redefs
                #(#stmts)*
            });
        )
    }

    /// Expand the modifies closure if we are handling a modifies attribute. Otherwise, no-op.
    pub fn expand_modifies(&self, closure_stmt: &mut Stmt) {
        if matches!(&self.condition_type, ContractConditionsData::Modifies { .. }) {
            let Stmt::Local(Local { init: Some(LocalInit { expr, .. }), .. }) = closure_stmt else {
                unreachable!()
            };
            // The closure is wrapped into `kani_force_fn_once_with_args`
            let Expr::Call(call) = expr.as_ref() else { unreachable!() };
            if call.args.len() != 1 {
                unreachable!()
            }
            let expr = call.args.first().unwrap();
            let Expr::Closure(closure) = expr else { unreachable!() };
            let Expr::Block(body) = closure.body.as_ref() else { unreachable!() };
            let stream = self.modifies_closure(&closure.output, &body.block, TokenStream2::new());
            *closure_stmt = syn::parse2(stream).unwrap();
        }
    }

    /// Return whether the original function has a mutable receiver.
    fn has_mutable_receiver(&self) -> bool {
        let first_arg = self.annotated_fn.sig.inputs.first();
        first_arg
            .map(|arg| {
                matches!(
                    arg,
                    FnArg::Receiver(syn::Receiver { mutability: Some(_), reference: None, .. },)
                )
            })
            .unwrap_or(false)
    }

    /// Return whether the original function takes `self` by value (an owned
    /// receiver), e.g. `fn into_inner(self)`.
    pub fn has_owned_receiver(&self) -> bool {
        let first_arg = self.annotated_fn.sig.inputs.first();
        first_arg
            .map(|arg| matches!(arg, FnArg::Receiver(syn::Receiver { reference: None, .. },)))
            .unwrap_or(false)
    }

    /// Generate argument re-definitions for arguments.
    ///
    /// This is used so Kani doesn't think that modifying a local argument value is a side effect,
    /// and avoid may-drop checks in const generic functions.
    pub fn arg_redefinitions(&self, redefine_only_mut: bool) -> TokenStream2 {
        let mut result = TokenStream2::new();
        for (mutability, ident) in self.arg_bindings() {
            if mutability == MutBinding::Mut {
                result.extend(quote!(let mut #ident = #ident;));
            } else if !redefine_only_mut {
                result.extend(quote!(let #ident = #ident;));
            }
        }
        result
    }

    /// Extract all arguments bindings and their mutability.
    fn arg_bindings(&self) -> impl Iterator<Item = (MutBinding, &Ident)> {
        self.annotated_fn.sig.inputs.iter().flat_map(|arg| match arg {
            FnArg::Receiver(_) => vec![],
            FnArg::Typed(typed) => pat_to_bindings(typed.pat.as_ref()),
        })
    }
}

/// A compile error if `body` already records a clause of the same panic kind as `condition`.
///
/// `contract_main` rejects a second clause by its attribute name, but a renamed import (`use
/// kani::panics_if as p;`) escapes that check. The expansion of the first clause stays in the
/// check closure, so look for its call to the matching `set_*` function instead.
fn duplicate_panic_clause(
    condition: &ContractConditionsData,
    body: &[Stmt],
) -> Option<TokenStream2> {
    let (setter, clause) = match condition {
        ContractConditionsData::PanicsOnlyIf { .. } => ("set_panics_only_if", "panics_only_if"),
        ContractConditionsData::PanicsIf { .. } => ("set_panics_if", "panics_if"),
        _ => return None,
    };
    let records_clause = body
        .iter()
        .any(|stmt| quote!(#stmt).into_iter().any(|token| token_mentions(&token, setter)));
    records_clause.then(|| {
        let message = format!("a function may have at most one `#[kani::{clause}]` clause");
        let error = syn::Error::new(Span::call_site(), message).into_compile_error();
        // The result replaces the check closure's body, which must be a block.
        quote!({ #error })
    })
}

/// Whether `token`, or any token nested in it, is the identifier `name`.
fn token_mentions(token: &proc_macro2::TokenTree, name: &str) -> bool {
    match token {
        proc_macro2::TokenTree::Ident(ident) => ident == name,
        proc_macro2::TokenTree::Group(group) => {
            group.stream().into_iter().any(|token| token_mentions(&token, name))
        }
        _ => false,
    }
}
