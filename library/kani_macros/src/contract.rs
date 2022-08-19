// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Utility functions for macro expansion of function contract clauses.
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{Attribute, Expr, FnArg, ItemFn, Signature};
use uuid::Uuid;

/// Given a function `foo`, this function creates a closure with name
/// `foo_<uuid>`, where <uuid> is a unique identifier.
/// The closure has the same function arguments and the same function body
/// as the original function `foo`.
pub fn convert_to_closure(item: &ItemFn) -> (Ident, TokenStream2) {
    let fn_sig = &item.sig;
    let body = &item.block;
    let args_with_ty = &fn_sig.inputs;
    let rt = &fn_sig.output;
    let name = format!("{}_{}", fn_sig.ident.clone().to_string(), Uuid::new_v4()).replace("-", "_");
    let ident = Ident::new(name.as_str(), Span::call_site());
    let inner_fn = quote! {
        let #ident = |#args_with_ty| #rt #body;
    };
    (ident, inner_fn)
}

/// Converts all "#[kani::ensures(...)]" attributes to "kani::postcondition(...)" statement tokens.
pub fn extract_ensures_as_postconditions(attributes: &Vec<Attribute>) -> TokenStream2 {
    attributes
        .iter()
        .filter_map(|a| {
            let name = a.path.segments.last().unwrap().ident.to_string();
            match name.as_str() {
                "ensures" => {
                    let arg = a
                        .parse_args::<Expr>()
                        .expect("An argument expected inside the ensures clause");
                    Some(quote! {kani::postcondition(#arg);})
                }
                _ => None,
            }
        })
        .collect()
}

/// Returns the list of function arguments from the function signature.
pub fn extract_function_args(sig: &Signature) -> Vec<syn::Pat> {
    sig.inputs
        .iter()
        .filter_map(|x| match x {
            FnArg::Typed(syn::PatType { pat, .. }) => Some(*pat.clone()),
            FnArg::Receiver(syn::Receiver { .. }) => None, // Ignore arguments like "self", etc.
        })
        .collect()
}

/// Return all attributes that are not inlined during macro expansion
/// (that is, not "[#kani::requires(...)]" or "[#kani::ensures(...)]").
pub fn extract_non_inlined_attributes(attributes: &Vec<Attribute>) -> TokenStream2 {
    attributes
        .iter()
        .filter_map(|a| {
            let name = a.path.segments.last().unwrap().ident.to_string();
            match name.as_str() {
                "ensures" | "requires" => None,
                _ => Some(quote! {#a}),
            }
        })
        .collect()
}

/// Converts all "#[kani::requires(...)]" attributes to "kani::precondition(...)" statement tokens.
pub fn extract_requires_as_preconditions(attributes: &Vec<Attribute>) -> TokenStream2 {
    attributes
        .iter()
        .filter_map(|a| {
            let name = a.path.segments.last().unwrap().ident.to_string();
            match name.as_str() {
                "requires" => {
                    let arg = a
                        .parse_args::<Expr>()
                        .expect("An argument expected inside the requires clause");
                    Some(quote!(kani::precondition(#arg);))
                }
                _ => None,
            }
        })
        .collect()
}

/// Splits a vector of attributes into a vector of "#[kani::modifies(...)]" attributes only and the rest.
pub fn handle_modifies_attributes(attributes: &Vec<Attribute>) -> (TokenStream2, TokenStream2) {
    let modifies_attrs = attributes
        .iter()
        .filter_map(|a| {
            let name = a.path.segments.last().unwrap().ident.to_string();
            match name.as_str() {
                "modifies" => Some(quote! {#a}),
                _ => None,
            }
        })
        .fold(quote! {}, |acc, new| quote! {#acc #new});
    let non_modifies_attrs = attributes
        .iter()
        .filter_map(|a| {
            let name = a.path.segments.last().unwrap().ident.to_string();
            match name.as_str() {
                "modifies" => None,
                _ => Some(quote! {#a}),
            }
        })
        .fold(quote! {}, |acc, new| quote! {#acc #new});
    (modifies_attrs, non_modifies_attrs)
}
