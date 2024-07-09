// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic used for generating the code that checks a contract.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use std::mem;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parse_quote, Block, Expr, FnArg, Local, LocalInit, Pat, ReturnType, Stmt};

use super::{
    helpers::*,
    shared::{build_ensures, try_as_result_assign_mut},
    ContractConditionsData, ContractConditionsHandler, INTERNAL_RESULT_IDENT,
};

const WRAPPER_ARG_PREFIX: &str = "_wrapper_arg_";

impl<'a> ContractConditionsHandler<'a> {
    /// Create the body of a check function.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    ///
    /// Mutable because a `modifies` clause may need to extend the inner call to
    /// the wrapper with new arguments.
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
                let wrapper_name = &self.modify_name;

                let wrapper_args = if let Some(wrapper_call_args) = body_stmts
                    .iter_mut()
                    .find_map(|stmt| try_as_wrapper_call_args(stmt, &wrapper_name))
                {
                    let wrapper_args = make_wrapper_idents(
                        wrapper_call_args.len(),
                        attr.len(),
                        WRAPPER_ARG_PREFIX,
                    );
                    wrapper_call_args
                        .extend(wrapper_args.clone().map(|a| Expr::Verbatim(quote!(#a))));
                    wrapper_args
                } else {
                    unreachable!("Expected check function to call to the modifies wrapper function")
                };

                quote!({
                    // Cast to *const () since we only care about the address.
                    #(let #wrapper_args = #attr as *const _ as *const ();)*
                    #(#body_stmts)*
                })
            }
        }
    }

    /// Initialize the list of statements for the check closure body.
    fn initial_check_stmts(&self) -> Vec<syn::Stmt> {
        let modifies_ident = Ident::new(&self.modify_name, Span::call_site());
        let return_type = return_type_to_type(&self.annotated_fn.sig.output);
        let modifies_closure = self.modifies_closure(
            &self.annotated_fn.sig.inputs,
            &self.annotated_fn.sig.output,
            &self.annotated_fn.block,
        );
        let (_, args) = closure_args(&self.annotated_fn.sig.inputs);
        let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
        parse_quote!(
            #modifies_closure
            let #result : #return_type = #modifies_ident(#(#args),*);
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
        let (inputs, _args) = closure_args(&sig.inputs);
        let output = &sig.output;
        let body_stmts = self.initial_check_stmts();
        let body = self.make_check_body(body_stmts);

        quote!(
            #[kanitool::is_contract_generated(check)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #check_ident = |#inputs| #output #body;
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

    /// Emit a modifies wrapper, possibly augmenting a prior, existing one.
    ///
    /// We only augment if this clause is a `modifies` clause. Before,
    /// we annotated the wrapper arguments with `impl kani::Arbitrary`,
    /// so Rust would infer the proper types for each argument.
    /// We want to remove the restriction that these arguments must
    /// implement `kani::Arbitrary` for checking. Now, we annotate each
    /// argument with a generic type parameter, so the compiler can
    /// continue inferring the correct types.
    pub fn modifies_closure(
        &self,
        inputs: &Punctuated<FnArg, Comma>,
        output: &ReturnType,
        body: &Block,
    ) -> TokenStream2 {
        // Filter receiver
        let (inputs, _args) = closure_args(inputs);
        let wrapper_args: Vec<_> =
            if let ContractConditionsData::Modifies { attr } = &self.condition_type {
                make_wrapper_idents(inputs.len(), attr.len(), WRAPPER_ARG_PREFIX).collect()
            } else {
                make_wrapper_idents(inputs.len(), 0, WRAPPER_ARG_PREFIX).collect()
            };
        let modifies_ident = Ident::new(&self.modify_name, Span::call_site());
        quote!(
            #[kanitool::is_contract_generated(wrapper)]
            #(#[kanitool::modifies(#wrapper_args)])*
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #modifies_ident = |#inputs #(, #wrapper_args: *const ())*| #output {
                #body
            };
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
            let inputs = closure
                .inputs
                .iter()
                .map(|pat| {
                    if let Pat::Type(pat_type) = pat {
                        FnArg::Typed(pat_type.clone())
                    } else {
                        panic!("Expected closure argument, but found: {pat:?}")
                    }
                })
                .collect();
            let stream = self.modifies_closure(&inputs, &closure.output, &body.block);
            *closure_stmt = syn::parse2(stream).unwrap();
        }
    }
}

/// Try to interpret this statement as `let result : <...> = <wrapper_fn_name>(args ...);` and
/// return a mutable reference to the parameter list.
fn try_as_wrapper_call_args<'a>(
    stmt: &'a mut syn::Stmt,
    wrapper_fn_name: &str,
) -> Option<&'a mut syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>> {
    let syn::LocalInit { diverge: None, expr: init_expr, .. } = try_as_result_assign_mut(stmt)?
    else {
        return None;
    };

    match init_expr.as_mut() {
        Expr::Call(syn::ExprCall { func: box_func, args, .. }) => match box_func.as_ref() {
            syn::Expr::Path(syn::ExprPath { qself: None, path, .. })
                if path.get_ident().map_or(false, |id| id == wrapper_fn_name) =>
            {
                Some(args)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Make `num` [`Ident`]s with the names `prefix{i}` with `i` starting at `low` and
/// increasing by one each time.
fn make_wrapper_idents(
    low: usize,
    num: usize,
    prefix: &'static str,
) -> impl Iterator<Item = syn::Ident> + Clone + 'static {
    (low..).map(move |i| Ident::new(&format!("{prefix}{i}"), Span::mixed_site())).take(num)
}
