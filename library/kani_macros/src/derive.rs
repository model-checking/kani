// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module provides Kani's `derive` macro for `Arbitrary`.
//!
//! ```
//! use kani::Arbitrary;
//!
//! #[derive(Arbitrary)]
//! struct S;
//!
//! ```
use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_error::abort;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Data, DataEnum, DeriveInput, Fields, GenericParam, Generics,
    Index,
};

pub fn expand_derive_arbitrary(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_item = parse_macro_input!(item as DeriveInput);
    let item_name = &derive_item.ident;

    // Add a bound `T: Arbitrary` to every type parameter T.
    let generics = add_trait_bound_arbitrary(derive_item.generics);
    // Generate an expression to sum up the heap size of each field.
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let body = fn_any_body(&item_name, &derive_item.data);
    let expanded = quote! {
        // The generated implementation.
        impl #impl_generics kani::Arbitrary for #item_name #ty_generics #where_clause {
            fn any() -> Self {
                #body
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

/// Add a bound `T: Arbitrary` to every type parameter T.
fn add_trait_bound_arbitrary(mut generics: Generics) -> Generics {
    generics.params.iter_mut().for_each(|param| {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(kani::Arbitrary));
        }
    });
    generics
}

/// Generate the body of the function `any()`.
/// This will create the non-deterministic object.
/// E.g.:
/// ```
/// #[derive(Arbitrary)]
/// struct Point { x: u8, y: u8 }
/// ```
/// will generate the following body for `fn any()`:
/// ```
/// fn any() -> Self {
///    Self { x: kani::any(), y: kani::any() }
/// }
/// ```
fn fn_any_body(ident: &Ident, data: &Data) -> TokenStream {
    match data {
        Data::Struct(struct_data) => init_symbolic_item(ident, &struct_data.fields),
        Data::Enum(enum_data) => fn_any_enum(ident, enum_data),
        Data::Union(_) => {
            abort!(Span::call_site(), "Cannot derive `Arbitrary` for `{}` union", ident;
                note = ident.span() =>
                "`#[derive(Arbitrary)]` cannot be used for unions such as `{}`", ident
            )
        }
    }
}

/// Generate an item initialization where an item can be a struct or a variant.
/// For named fields, this will generate: `Item { field1: kani::any(), field2: kani::any(), .. }`
/// For unnamed fields, this will generate: `Item (kani::any(), kani::any(), ..)`
/// For unit field, generate an empty initialization.
fn init_symbolic_item(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            // Use the span of each `syn::Field`. This way if one of the field types does not
            // implement `Arbitrary` then the compiler's error message underlines which field it
            // is. An example is shown in the readme of the parent directory.
            let init = fields.named.iter().map(|field| {
                let name = &field.ident;
                quote_spanned! {field.span()=>
                    #name: kani::any()
                }
            });
            quote! {
                #ident {#( #init,)*}
            }
        }
        Fields::Unnamed(ref fields) => {
            // Expands to an expression like
            // Self(kani::any(), kani::any(), ..., kani::any());
            let init = fields.unnamed.iter().map(|field| {
                quote_spanned! {field.span()=>
                    kani::any()
                }
            });
            quote! {
                #ident(#( #init,)*)
            }
        }
        Fields::Unit => {
            quote! {
                #ident
            }
        }
    }
}

/// Generate the body of the function `any()` for enums. The cases are:
/// 1. For zero-variants enumerations, this will encode a `panic!()` statement.
/// 2. For one or more variants, the code will be something like:
/// ```
/// # enum Enum{
/// #    WithoutData,
/// #    WithUnNamedData(i32),
/// #    WithNamedData{ i: i32},
/// # }
/// #
/// # impl kani::Arbitrary for Enum {
/// #     fn any() -> Self {
///         match kani::any() {
///             0 => Enum::WithoutData,
///             1 => Enum::WithUnNamedData(kani::any()),
///             _ => Enum::WithNamedData {i: kani::any()},
///         }
/// #    }
/// # }
/// ```
fn fn_any_enum(ident: &Ident, data: &DataEnum) -> TokenStream {
    if data.variants.is_empty() {
        let msg = format!(
            "Cannot create symbolic enum `{ident}`. Enums with zero-variants cannot be instantiated"
        );
        quote! {
            panic!(#msg)
        }
    } else {
        let arms = data.variants.iter().enumerate().map(|(idx, variant)| {
            let init = init_symbolic_item(&variant.ident, &variant.fields);
            if idx + 1 < data.variants.len() {
                let index = Index::from(idx);
                quote! {
                    #index => #ident::#init,
                }
            } else {
                quote! {
                    _ => #ident::#init,
                }
            }
        });

        quote! {
            match kani::any() {
                #(#arms)*
            }
        }
    }
}

pub fn expand_derive_invariant(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_item = parse_macro_input!(item as DeriveInput);
    let item_name = &derive_item.ident;

    // Add a bound `T: Invariant` to every type parameter T.
    let generics = add_trait_bound_invariant(derive_item.generics);
    // Generate an expression to sum up the heap size of each field.
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let body = is_safe_body(&item_name, &derive_item.data);
    let expanded = quote! {
        // The generated implementation.
        impl #impl_generics kani::Invariant for #item_name #ty_generics #where_clause {
            fn is_safe(&self) -> bool {
                #body
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

/// Add a bound `T: Invariant` to every type parameter T.
fn add_trait_bound_invariant(mut generics: Generics) -> Generics {
    generics.params.iter_mut().for_each(|param| {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(kani::Invariant));
        }
    });
    generics
}

fn is_safe_body(ident: &Ident, data: &Data) -> TokenStream {
    match data {
        Data::Struct(struct_data) => struct_safe_conjunction(ident, &struct_data.fields),
        Data::Enum(_) => {
            abort!(Span::call_site(), "Cannot derive `Invariant` for `{}` enum", ident;
                note = ident.span() =>
                "`#[derive(Invariant)]` cannot be used for enums such as `{}`", ident
            )
        }
        Data::Union(_) => {
            abort!(Span::call_site(), "Cannot derive `Invariant` for `{}` union", ident;
                note = ident.span() =>
                "`#[derive(Invariant)]` cannot be used for unions such as `{}`", ident
            )
        }
    }
}

/// Generates an expression that is the conjunction of `is_safe` calls for each field in the struct.
fn struct_safe_conjunction(_ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        // Expands to the expression
        // `true && self.field1.is_safe() && self.field2.is_safe() && ..`
        Fields::Named(ref fields) => {
            let safe_calls = fields.named.iter().map(|field| {
                let name = &field.ident;
                quote_spanned! {field.span()=>
                    self.#name.is_safe()
                }
            });
            // An initial value is required for empty structs
            safe_calls.fold(quote! { true }, |acc, call| {
                quote! { #acc && #call }
            })
        }
        Fields::Unnamed(ref fields) => {
            // Expands to the expression
            // `true && self.0.is_safe() && self.1.is_safe() && ..`
            let safe_calls = fields.unnamed.iter().enumerate().map(|(i, field)| {
                let idx = syn::Index::from(i);
                quote_spanned! {field.span()=>
                    self.#idx.is_safe()
                }
            });
            // An initial value is required for empty structs
            safe_calls.fold(quote! { true }, |acc, call| {
                quote! { #acc && #call }
            })
        }
        // Expands to the expression
        // `true`
        Fields::Unit => {
            quote! {
                true
            }
        }
    }
}
