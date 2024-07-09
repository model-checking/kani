// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic used for generating the code that replaces a function with its contract.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use std::mem;
use syn::Stmt;

use super::{
    helpers::*,
    shared::{build_ensures, try_as_result_assign},
    ContractConditionsData, ContractConditionsHandler, INTERNAL_RESULT_IDENT,
};

impl<'a> ContractConditionsHandler<'a> {
    /// Create initial set of replace statements which is the return havoc.
    fn initial_replace_stmts(&self) -> Vec<syn::Stmt> {
        let return_type = return_type_to_type(&self.annotated_fn.sig.output);
        let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
        vec![syn::parse_quote!(let #result : #return_type = kani::any_modifies();)]
    }

    /// Split an existing replace body of the form
    ///
    /// ```ignore
    /// // multiple preconditions and argument copies like like
    /// kani::assert(.. precondition);
    /// let arg_name = kani::internal::untracked_deref(&arg_value);
    /// // single result havoc
    /// let result : ResultType = kani::any();
    ///
    /// // multiple argument havockings
    /// *unsafe { kani::internal::Pointer::assignable(argument) } = kani::any();
    /// // multiple postconditions
    /// kani::assume(postcond);
    /// // multiple argument copy (used in postconditions) cleanups
    /// std::mem::forget(arg_name);
    /// // single return
    /// result
    /// ```
    ///
    /// Such that the first vector contains everything up to and including the single result havoc
    /// and the second one the rest, excluding the return.
    ///
    fn split_replace(&self, mut stmts: Vec<Stmt>) -> (Vec<Stmt>, Vec<Stmt>) {
        // Pop the return result since we always re-add it.
        stmts.pop();

        let idx = stmts
            .iter()
            .enumerate()
            .find_map(|(i, elem)| is_replace_return_havoc(elem).then_some(i))
            .unwrap_or_else(|| {
                panic!("ICE: Could not find result let binding in statement sequence")
            });
        // We want the result assign statement to end up as the last statement in the first
        // vector, hence the `+1`.
        let (before, after) = stmts.split_at_mut(idx + 1);
        (before.to_vec(), after.to_vec())
    }

    /// Create the body of a stub for this contract.
    ///
    /// Wraps the conditions from this attribute around a prior call. If
    /// `use_nondet_result` is `true` we will use `kani::any()` to create a
    /// result, otherwise whatever the `body` of our annotated function was.
    ///
    /// `use_nondet_result` will only be true if this is the first time we are
    /// generating a replace function.
    fn expand_replace_body(&self, before: &[Stmt], after: &[Stmt]) -> TokenStream {
        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                let Self { attr_copy, .. } = self;
                let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
                quote!({
                    kani::assert(#attr, stringify!(#attr_copy));
                    #(#before)*
                    #(#after)*
                    #result
                })
            }
            ContractConditionsData::Ensures { attr } => {
                let (remembers, ensures_clause) = build_ensures(attr);
                let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
                quote!({
                    #remembers
                    #(#before)*
                    #(#after)*
                    kani::assume(#ensures_clause);
                    #result
                })
            }
            ContractConditionsData::Modifies { attr } => {
                let result = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
                quote!({
                    #(#before)*
                    #(*unsafe { kani::internal::Pointer::assignable(kani::internal::untracked_deref(&(#attr))) } = kani::any_modifies();)*
                    #(#after)*
                    #result
                })
            }
        }
    }

    /// Emit the replace function into the output stream.
    ///
    /// See [`Self::expand_replace_body`] for the most interesting parts of this
    /// function.
    pub fn replace_closure(&self) -> TokenStream {
        let replace_ident = Ident::new(&self.replace_name, Span::call_site());
        let sig = &self.annotated_fn.sig;
        let (inputs, _args) = closure_args(&sig.inputs);
        let output = &sig.output;
        let before = self.initial_replace_stmts();
        let body = self.expand_replace_body(&before, &vec![]);

        quote!(
            #[kanitool::is_contract_generated(replace)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #replace_ident = |#inputs| #output #body;
        )
    }

    /// Expand the `replace` body with the new attribute.
    pub fn expand_replace(&self, closure: &mut Stmt) {
        let body = closure_body(closure);
        let (before, after) = self.split_replace(mem::take(&mut body.block.stmts));
        let stream = self.expand_replace_body(&before, &after);
        *body = syn::parse2(stream).unwrap();
    }
}

/// Is this statement `let result_kani_internal : <...> = kani::any_modifies();`.
fn is_replace_return_havoc(stmt: &syn::Stmt) -> bool {
    let Some(syn::LocalInit { diverge: None, expr: e, .. }) = try_as_result_assign(stmt) else {
        return false;
    };

    matches!(
        e.as_ref(),
        syn::Expr::Call(syn::ExprCall {
            func,
            args,
            ..
        })
        if args.is_empty()
        && matches!(
            func.as_ref(),
            syn::Expr::Path(syn::ExprPath {
                qself: None,
                path,
                attrs,
            })
            if path.segments.len() == 2
            && path.segments[0].ident == "kani"
            && path.segments[1].ident == "any_modifies"
            && attrs.is_empty()
        )
    )
}
