// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic used for generating the code that replaces a function with its contract.

use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;

use super::{
    helpers::*,
    shared::{make_unsafe_argument_copies, try_as_result_assign},
    ContractConditionsData, ContractConditionsHandler,
};

impl<'a> ContractConditionsHandler<'a> {
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
    /// If this is the first time we're emitting replace we create the return havoc and nothing else.
    fn ensure_bootstrapped_replace_body(&self) -> (Vec<syn::Stmt>, Vec<syn::Stmt>) {
        if self.is_first_emit() {
            let return_type = return_type_to_type(&self.annotated_fn.sig.output);
            (vec![syn::parse_quote!(let result : #return_type = kani::any_modifies();)], vec![])
        } else {
            let stmts = &self.annotated_fn.block.stmts;
            let idx = stmts
                .iter()
                .enumerate()
                .find_map(|(i, elem)| is_replace_return_havoc(elem).then_some(i))
                .unwrap_or_else(|| {
                    panic!("ICE: Could not find result let binding in statement sequence")
                });
            // We want the result assign statement to end up as the last statement in the first
            // vector, hence the `+1`.
            let (before, after) = stmts.split_at(idx + 1);
            (before.to_vec(), after.split_last().unwrap().1.to_vec())
        }
    }

    /// Create the body of a stub for this contract.
    ///
    /// Wraps the conditions from this attribute around a prior call. If
    /// `use_nondet_result` is `true` we will use `kani::any()` to create a
    /// result, otherwise whatever the `body` of our annotated function was.
    ///
    /// `use_nondet_result` will only be true if this is the first time we are
    /// generating a replace function.
    fn make_replace_body(&self) -> TokenStream2 {
        let (before, after) = self.ensure_bootstrapped_replace_body();

        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                let Self { attr_copy, .. } = self;
                quote!(
                    kani::assert(#attr, stringify!(#attr_copy));
                    #(#before)*
                    #(#after)*
                    result
                )
            }
            ContractConditionsData::Ensures { attr, argument_names } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);
                quote!(
                    #arg_copies
                    #(#before)*
                    #(#after)*
                    kani::assume(#attr);
                    #copy_clean
                    result
                )
            }
            ContractConditionsData::Modifies { attr } => {
                quote!(
                    #(#before)*
                    #(*unsafe { kani::internal::Pointer::assignable(#attr) } = kani::any_modifies();)*
                    #(#after)*
                    result
                )
            }
        }
    }

    /// Emit the replace funtion into the output stream.
    ///
    /// See [`Self::make_replace_body`] for the most interesting parts of this
    /// function.
    pub fn emit_replace_function(&mut self, override_function_ident: Option<Ident>) {
        self.emit_common_header();

        if self.function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            self.output.extend(quote!(#[kanitool::is_contract_generated(replace)]));
        }
        let mut sig = self.annotated_fn.sig.clone();
        let body = self.make_replace_body();
        if let Some(ident) = override_function_ident {
            sig.ident = ident;
        }

        // Finally emit the check function itself.
        self.output.extend(quote!(
            #sig {
                #body
            }
        ));
    }
}

/// Is this statement `let result : <...> = kani::any_modifies();`.
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
