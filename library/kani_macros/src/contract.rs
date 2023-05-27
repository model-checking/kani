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
    let name = format!("{}_{}", fn_sig.ident.clone().to_string(), Uuid::new_v4()).replace('-', "_");
    let ident = Ident::new(name.as_str(), Span::call_site());
    let inner_fn = quote! {
        let #ident = |#args_with_ty| #rt #body;
    };
    (ident, inner_fn)
}

/// If the attribute is named `kani::name`, this extracts `name`
fn kani_attr_name(attr: &Attribute) -> Option<String> {
    let segments = &attr.path.segments;
    if segments.len() == 2 && segments[0].ident == "kani" {
        Some(segments[1].ident.to_string())
    } else {
        None
    }
}

/// Converts all "#[kani::ensures(...)]" attributes to "kani::postcondition(...)" statement tokens.
pub fn extract_ensures_as_postconditions(attributes: &[Attribute]) -> TokenStream2 {
    attributes
        .iter()
        .filter_map(|a| {
            let name = kani_attr_name(a);
            match name.as_deref() {
                Some("ensures") => {
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

/// Returns the list of function arguments from the function signature
///     for use in a function or method call.
pub fn extract_function_args(sig: &Signature) -> Vec<syn::Pat> {
    sig.inputs
        .iter()
        .filter_map(|x| match x {
            FnArg::Typed(syn::PatType { pat, .. }) => Some(*pat.clone()),
            // Ignore the "self" argument of an associated method while calling the method.
            // For example, `vec![x, y]` is extracted from `fn foo(self, x: i32, y: i32)`
            //   for use in the method call - `foo(x, y);`
            FnArg::Receiver(syn::Receiver { .. }) => None,
        })
        .collect()
}

/// Return all attributes that are not inlined during macro expansion
/// (that is, not "#[kani::requires(...)]" or "#[kani::ensures(...)]").
pub fn extract_non_inlined_attributes(attributes: &[Attribute]) -> TokenStream2 {
    attributes
        .iter()
        .filter_map(|a| {
            let name = kani_attr_name(a);
            match name.as_deref() {
                Some("requires") | Some("ensures") => None,
                _ => Some(quote! {#a}),
            }
        })
        .collect()
}

/// Converts all "#[kani::requires(...)]" attributes to "kani::precondition(...)" statement tokens.
pub fn extract_requires_as_preconditions(attributes: &[Attribute]) -> TokenStream2 {
    attributes
        .iter()
        .filter_map(|a| {
            let name = kani_attr_name(a);
            match name.as_deref() {
                Some("requires") => {
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
pub fn handle_modifies_attributes(attributes: &[Attribute]) -> (TokenStream2, TokenStream2) {
    let modifies_attrs = attributes
        .iter()
        .filter_map(|a| {
            let name = kani_attr_name(a);
            match name.as_deref() {
                Some("modifies") => Some(quote! {#a}),
                _ => None,
            }
        })
        .fold(quote! {}, |acc, new| quote! {#acc #new});
    let non_modifies_attrs = attributes
        .iter()
        .filter_map(|a| {
            let name = kani_attr_name(a);
            match name.as_deref() {
                Some("modifies") => None,
                _ => Some(quote! {#a}),
            }
        })
        .fold(quote! {}, |acc, new| quote! {#acc #new});
    (modifies_attrs, non_modifies_attrs)
}
