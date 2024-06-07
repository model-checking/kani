// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic that is shared between [`super::initialize`], [`super::check`] and
//! [`super::replace`].
//!
//! This is so we can keep [`super`] distraction-free as the definitions of data
//! structures and the entry point for contract handling.

use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::Attribute;

use super::{ContractConditionsHandler, ContractFunctionState, INTERNAL_RESULT_IDENT};

impl ContractFunctionState {
    /// Do we need to emit the `is_contract_generated` tag attribute on the
    /// generated function(s)?
    pub fn emit_tag_attr(self) -> bool {
        matches!(self, ContractFunctionState::Untouched)
    }
}

impl<'a> ContractConditionsHandler<'a> {
    pub fn is_first_emit(&self) -> bool {
        matches!(self.function_state, ContractFunctionState::Untouched)
    }

    /// Create a new name for the assigns wrapper function *or* get the name of
    /// the wrapper we must have already generated. This is so that we can
    /// recognize a call to that wrapper inside the check function.
    pub fn make_wrapper_name(&self) -> Ident {
        if let Some(hash) = self.hash {
            identifier_for_generated_function(&self.annotated_fn.sig.ident, "wrapper", hash)
        } else {
            let str_name = self.annotated_fn.sig.ident.to_string();
            let splits = str_name.rsplitn(3, '_').collect::<Vec<_>>();
            let [hash, _, base] = splits.as_slice() else {
                unreachable!("Odd name for function {str_name}, splits were {}", splits.len());
            };

            Ident::new(&format!("{base}_wrapper_{hash}"), Span::call_site())
        }
    }

    /// Emit attributes common to check or replace function into the output
    /// stream.
    pub fn emit_common_header(&mut self) {
        if self.function_state.emit_tag_attr() {
            self.output.extend(quote!(
                #[allow(dead_code, unused_variables)]
            ));
        }
        self.output.extend(self.annotated_fn.attrs.iter().flat_map(Attribute::to_token_stream));
    }
}

/// Makes consistent names for a generated function which was created for
/// `purpose`, from an attribute that decorates `related_function` with the
/// hash `hash`.
pub fn identifier_for_generated_function(
    related_function_name: &Ident,
    purpose: &str,
    hash: u64,
) -> Ident {
    let identifier = format!("{}_{purpose}_{hash:x}", related_function_name);
    Ident::new(&identifier, proc_macro2::Span::mixed_site())
}

/// We make shallow copies of the argument for the postconditions in both
/// `requires` and `ensures` clauses and later clean them up.
///
/// This function creates the code necessary to both make the copies (first
/// tuple elem) and to clean them (second tuple elem).
pub fn make_unsafe_argument_copies(
    renaming_map: &HashMap<Ident, Ident>,
) -> (TokenStream2, TokenStream2) {
    let arg_names = renaming_map.values();
    let also_arg_names = renaming_map.values();
    let arg_values = renaming_map.keys();
    (
        quote!(#(let #arg_names = kani::internal::untracked_deref(&#arg_values);)*),
        #[cfg(not(feature = "no_core"))]
        quote!(#(std::mem::forget(#also_arg_names);)*),
        #[cfg(feature = "no_core")]
        quote!(#(core::mem::forget(#also_arg_names);)*),
    )
}

/// Used as the "single source of truth" for [`try_as_result_assign`] and [`try_as_result_assign_mut`]
/// since we can't abstract over mutability. Input is the object to match on and the name of the
/// function used to convert an `Option<LocalInit>` into the result type (e.g. `as_ref` and `as_mut`
/// respectively).
///
/// We start with a `match` as a top-level here, since if we made this a pattern macro (the "clean"
/// thing to do) then we cant use the `if` inside there which we need because box patterns are
/// unstable.
macro_rules! try_as_result_assign_pat {
    ($input:expr, $convert:ident) => {
        match $input {
            syn::Stmt::Local(syn::Local {
                pat: syn::Pat::Type(syn::PatType {
                    pat: inner_pat,
                    attrs,
                    ..
                }),
                init,
                ..
            }) if attrs.is_empty()
            && matches!(
                inner_pat.as_ref(),
                syn::Pat::Ident(syn::PatIdent {
                    by_ref: None,
                    mutability: None,
                    ident: result_ident,
                    subpat: None,
                    ..
                }) if result_ident == INTERNAL_RESULT_IDENT
            ) => init.$convert(),
            _ => None,
        }
    };
}

/// Try to parse this statement as `let result : <...> = <init>;` and return `init`.
///
/// This is the shape of statement we create in replace functions to havoc (with `init` being
/// `kani::any()`) and we need to recognize it for when we edit the replace function and integrate
/// additional conditions.
///
/// It's a thin wrapper around [`try_as_result_assign_pat!`] to create an immutable match.
pub fn try_as_result_assign(stmt: &syn::Stmt) -> Option<&syn::LocalInit> {
    try_as_result_assign_pat!(stmt, as_ref)
}

/// Try to parse this statement as `let result : <...> = <init>;` and return a mutable reference to
/// `init`.
///
/// This is the shape of statement we create in check functions (with `init` being a call to check
/// function with additional pointer arguments for the `modifies` clause) and we need to recognize
/// it to then edit this call if we find another `modifies` clause and add its additional arguments.
/// additional conditions.
///
/// It's a thin wrapper around [`try_as_result_assign_pat!`] to create a mutable match.
pub fn try_as_result_assign_mut(stmt: &mut syn::Stmt) -> Option<&mut syn::LocalInit> {
    try_as_result_assign_pat!(stmt, as_mut)
}
