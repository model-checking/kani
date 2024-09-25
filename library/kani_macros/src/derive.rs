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
use proc_macro_error2::abort;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Data, DataEnum, DeriveInput, Fields, GenericParam, Generics,
    Index,
};

/// Generate the Arbitrary implementation for the given type.
///
/// Note that we cannot use `proc_macro_crate::crate_name()` to discover the name for `kani` crate
/// since we define it as an extern crate via `rustc` command line.
///
/// In order to support core, we check the `no_core` feature.
pub fn expand_derive_arbitrary(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let trait_name = "Arbitrary";
    let derive_item = parse_macro_input!(item as DeriveInput);
    let item_name = &derive_item.ident;
    let kani_path = kani_path();

    let body = fn_any_body(&item_name, &derive_item.data);
    // Get the safety constraints (if any) to produce type-safe values
    let safety_conds_opt = safety_conds_opt(&item_name, &derive_item, trait_name);

    // Add a bound `T: Arbitrary` to every type parameter T.
    let generics = add_trait_bound_arbitrary(derive_item.generics);
    // Generate an expression to sum up the heap size of each field.
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = if let Some(safety_conds) = safety_conds_opt {
        let field_refs = field_refs(&item_name, &derive_item.data);
        quote! {
            // The generated implementation.
            impl #impl_generics #kani_path::Arbitrary for #item_name #ty_generics #where_clause {
                fn any() -> Self {
                    let obj = #body;
                    #field_refs
                    #kani_path::assume(#safety_conds);
                    obj
                }
            }
        }
    } else {
        quote! {
            // The generated implementation.
            impl #impl_generics #kani_path::Arbitrary for #item_name #ty_generics #where_clause {
                fn any() -> Self {
                    #body
                }
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

/// Add a bound `T: Arbitrary` to every type parameter T.
fn add_trait_bound_arbitrary(mut generics: Generics) -> Generics {
    let kani_path = kani_path();
    generics.params.iter_mut().for_each(|param| {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(#kani_path::Arbitrary));
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

/// Generates the sequence of expressions to initialize the variables used as
/// references to the struct fields.
///
/// For example, if we're deriving implementations for the struct
/// ```
/// #[derive(Arbitrary)]
/// #[derive(Invariant)]
/// struct PositivePoint {
///     #[safety_constraint(*x >= 0)]
///     x: i32,
///     #[safety_constraint(*y >= 0)]
///     y: i32,
/// }
/// ```
/// this function will generate the `TokenStream`
/// ```
/// let x = &obj.x;
/// let y = &obj.y;
/// ```
/// which allows us to refer to the struct fields without using `self`.
/// Note that the actual stream is generated in the `field_refs_inner` function.
fn field_refs(ident: &Ident, data: &Data) -> TokenStream {
    match data {
        Data::Struct(struct_data) => field_refs_inner(ident, &struct_data.fields),
        Data::Enum(_) => unreachable!(),
        Data::Union(_) => unreachable!(),
    }
}

/// Generates the sequence of expressions to initialize the variables used as
/// references to the struct fields. See `field_refs` for more details.
fn field_refs_inner(_ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            let field_refs: Vec<TokenStream> = fields
                .named
                .iter()
                .map(|field| {
                    let name = &field.ident;
                    quote_spanned! {field.span()=>
                        let #name = &obj.#name;
                    }
                })
                .collect();
            if !field_refs.is_empty() {
                quote! { #( #field_refs )* }
            } else {
                quote! {}
            }
        }
        Fields::Unnamed(_) => quote! {},
        Fields::Unit => quote! {},
    }
}

fn safe_body_default(ident: &Ident, data: &Data) -> TokenStream {
    match data {
        Data::Struct(struct_data) => safe_body_default_inner(ident, &struct_data.fields),
        Data::Enum(_) => unreachable!(),
        Data::Union(_) => unreachable!(),
    }
}

fn safe_body_default_inner(_ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        Fields::Named(ref fields) => {
            let field_safe_calls: Vec<TokenStream> = fields
                .named
                .iter()
                .map(|field| {
                    let name = &field.ident;
                    quote_spanned! {field.span()=>
                        #name.is_safe()
                    }
                })
                .collect();
            if !field_safe_calls.is_empty() {
                quote! { #( #field_safe_calls )&&* }
            } else {
                quote! { true }
            }
        }
        Fields::Unnamed(ref fields) => {
            let field_safe_calls: Vec<TokenStream> = fields
                .unnamed
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    let field_idx = Index::from(idx);
                    quote_spanned! {field.span()=>
                        #field_idx.is_safe()
                    }
                })
                .collect();
            if !field_safe_calls.is_empty() {
                quote! { #( #field_safe_calls )&&* }
            } else {
                quote! { true }
            }
        }
        Fields::Unit => quote! { true },
    }
}

/// Generate an item initialization where an item can be a struct or a variant.
/// For named fields, this will generate: `Item { field1: kani::any(), field2: kani::any(), .. }`
/// For unnamed fields, this will generate: `Item (kani::any(), kani::any(), ..)`
/// For unit field, generate an empty initialization.
fn init_symbolic_item(ident: &Ident, fields: &Fields) -> TokenStream {
    let kani_path = kani_path();
    match fields {
        Fields::Named(ref fields) => {
            // Use the span of each `syn::Field`. This way if one of the field types does not
            // implement `Arbitrary` then the compiler's error message underlines which field it
            // is. An example is shown in the readme of the parent directory.
            let init = fields.named.iter().map(|field| {
                let name = &field.ident;
                quote_spanned! {field.span()=>
                    #name: #kani_path::any()
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
                    #kani_path::any()
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

/// Extract, parse and return the expression `cond` (i.e., `Some(cond)`) in the
/// `#[safety_constraint(<cond>)]` attribute helper associated with a given field.
/// Return `None` if the attribute isn't specified.
fn parse_safety_expr(ident: &Ident, field: &syn::Field) -> Option<TokenStream> {
    let name = &field.ident;
    let mut safety_helper_attr = None;

    // Keep the helper attribute if we find it
    for attr in &field.attrs {
        if attr.path().is_ident("safety_constraint") {
            safety_helper_attr = Some(attr);
        }
    }

    // Parse the arguments in the `#[safety_constraint(...)]` attribute
    if let Some(attr) = safety_helper_attr {
        let expr_args: Result<syn::Expr, syn::Error> = attr.parse_args();

        // Check if there was an error parsing the arguments
        if let Err(err) = expr_args {
            abort!(Span::call_site(), "Cannot derive impl for `{}`", ident;
            note = attr.span() =>
            "safety constraint in field `{}` could not be parsed: {}", name.as_ref().unwrap().to_string(), err
            )
        }

        // Return the expression associated to the safety constraint
        let safety_expr = expr_args.unwrap();
        Some(quote_spanned! {field.span()=>
            #safety_expr
        })
    } else {
        None
    }
}

fn parse_safety_expr_input(ident: &Ident, derive_input: &DeriveInput) -> Option<TokenStream> {
    let name = ident;
    let mut safety_attr = None;

    // Keep the attribute if we find it
    for attr in &derive_input.attrs {
        if attr.path().is_ident("safety_constraint") {
            safety_attr = Some(attr);
        }
    }

    // Parse the arguments in the `#[safety_constraint(...)]` attribute
    if let Some(attr) = safety_attr {
        let expr_args: Result<syn::Expr, syn::Error> = attr.parse_args();

        // Check if there was an error parsing the arguments
        if let Err(err) = expr_args {
            abort!(Span::call_site(), "Cannot derive impl for `{}`", ident;
            note = attr.span() =>
            "safety constraint in `{}` could not be parsed: {}", name.to_string(), err
            )
        }

        // Return the expression associated to the safety constraint
        let safety_expr = expr_args.unwrap();
        Some(quote_spanned! {derive_input.span()=>
            #safety_expr
        })
    } else {
        None
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

        let kani_path = kani_path();
        quote! {
            match #kani_path::any() {
                #(#arms)*
            }
        }
    }
}

fn safe_body_with_calls(
    item_name: &Ident,
    derive_input: &DeriveInput,
    trait_name: &str,
) -> TokenStream {
    let safety_conds_opt = safety_conds_opt(&item_name, &derive_input, trait_name);
    let safe_body_default = safe_body_default(&item_name, &derive_input.data);

    if let Some(safety_conds) = safety_conds_opt {
        quote! { #safe_body_default && #safety_conds }
    } else {
        safe_body_default
    }
}

pub fn expand_derive_invariant(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let trait_name = "Invariant";
    let derive_item = parse_macro_input!(item as DeriveInput);
    let item_name = &derive_item.ident;
    let kani_path = kani_path();

    let safe_body = safe_body_with_calls(&item_name, &derive_item, trait_name);
    let field_refs = field_refs(&item_name, &derive_item.data);

    // Add a bound `T: Invariant` to every type parameter T.
    let generics = add_trait_bound_invariant(derive_item.generics);
    // Generate an expression to sum up the heap size of each field.
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        // The generated implementation.
        impl #impl_generics #kani_path::Invariant for #item_name #ty_generics #where_clause {
            fn is_safe(&self) -> bool {
                let obj = self;
                #field_refs
                #safe_body
            }
        }
    };
    proc_macro::TokenStream::from(expanded)
}

/// Looks for `#[safety_constraint(...)]` attributes used in the struct or its
/// fields, and returns the constraints if there were any, otherwise returns
/// `None`.
/// Note: Errors out if the attribute is used in both the struct and its fields.
fn safety_conds_opt(
    item_name: &Ident,
    derive_input: &DeriveInput,
    trait_name: &str,
) -> Option<TokenStream> {
    let has_item_safety_constraint =
        has_item_safety_constraint(&item_name, &derive_input, trait_name);

    let has_field_safety_constraints = has_field_safety_constraints(&item_name, &derive_input.data);

    if has_item_safety_constraint && has_field_safety_constraints {
        abort!(Span::call_site(), "Cannot derive `{}` for `{}`", trait_name, item_name;
        note = item_name.span() =>
        "`#[safety_constraint(...)]` cannot be used in struct and its fields simultaneously"
        )
    }

    if has_item_safety_constraint {
        Some(safe_body_from_struct_attr(&item_name, &derive_input, trait_name))
    } else if has_field_safety_constraints {
        Some(safe_body_from_fields_attr(&item_name, &derive_input.data, trait_name))
    } else {
        None
    }
}

fn has_item_safety_constraint(ident: &Ident, derive_input: &DeriveInput, trait_name: &str) -> bool {
    let safety_constraints_in_item =
        derive_input.attrs.iter().filter(|attr| attr.path().is_ident("safety_constraint")).count();
    if safety_constraints_in_item > 1 {
        abort!(Span::call_site(), "Cannot derive `{}` for `{}`", trait_name, ident;
        note = ident.span() =>
        "`#[safety_constraint(...)]` cannot be used more than once."
        )
    }
    safety_constraints_in_item == 1
}

fn has_field_safety_constraints(ident: &Ident, data: &Data) -> bool {
    match data {
        Data::Struct(struct_data) => has_field_safety_constraints_inner(ident, &struct_data.fields),
        Data::Enum(_) => false,
        Data::Union(_) => false,
    }
}

/// Checks if the `#[safety_constraint(...)]` attribute is attached to any
/// field.
fn has_field_safety_constraints_inner(_ident: &Ident, fields: &Fields) -> bool {
    match fields {
        Fields::Named(ref fields) => fields
            .named
            .iter()
            .any(|field| field.attrs.iter().any(|attr| attr.path().is_ident("safety_constraint"))),
        Fields::Unnamed(_) => false,
        Fields::Unit => false,
    }
}

/// Add a bound `T: Invariant` to every type parameter T.
pub fn add_trait_bound_invariant(mut generics: Generics) -> Generics {
    let kani_path = kani_path();
    generics.params.iter_mut().for_each(|param| {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(parse_quote!(#kani_path::Invariant));
        }
    });
    generics
}

fn safe_body_from_struct_attr(
    ident: &Ident,
    derive_input: &DeriveInput,
    trait_name: &str,
) -> TokenStream {
    if !matches!(derive_input.data, Data::Struct(_)) {
        abort!(Span::call_site(), "Cannot derive `{}` for `{}`", trait_name, ident;
            note = ident.span() =>
            "`#[safety_constraint(...)]` can only be used in structs"
        )
    };
    parse_safety_expr_input(ident, derive_input).unwrap()
}

/// Parse the condition expressions in `#[safety_constraint(<cond>)]` attached to struct
/// fields and, it at least one was found, generate a conjunction to be assumed.
///
/// For example, if we're deriving implementations for the struct
/// ```
/// #[derive(Arbitrary)]
/// #[derive(Invariant)]
/// struct PositivePoint {
///     #[safety_constraint(*x >= 0)]
///     x: i32,
///     #[safety_constraint(*y >= 0)]
///     y: i32,
/// }
/// ```
/// this function will generate the `TokenStream`
/// ```
/// *x >= 0 && *y >= 0
/// ```
/// which can be used by the `Arbitrary` and `Invariant` to generate and check
/// type-safe values for the struct, respectively.
fn safe_body_from_fields_attr(ident: &Ident, data: &Data, trait_name: &str) -> TokenStream {
    match data {
        Data::Struct(struct_data) => safe_body_from_fields_attr_inner(ident, &struct_data.fields),
        Data::Enum(_) => {
            abort!(Span::call_site(), "Cannot derive `{}` for `{}` enum", trait_name, ident;
                note = ident.span() =>
                "`#[derive(Invariant)]` cannot be used for enums such as `{}`", ident
            )
        }
        Data::Union(_) => {
            abort!(Span::call_site(), "Cannot derive `{}` for `{}` union", trait_name, ident;
                note = ident.span() =>
                "`#[derive(Invariant)]` cannot be used for unions such as `{}`", ident
            )
        }
    }
}

/// Generates an expression resulting from the conjunction of conditions
/// specified as safety constraints for each field.
/// See `safe_body_from_fields_attr` for more details.
fn safe_body_from_fields_attr_inner(ident: &Ident, fields: &Fields) -> TokenStream {
    match fields {
        // Expands to the expression
        // `<safety_cond1> && <safety_cond2> && ..`
        // where `<safety_condN>` is the safety condition specified for the N-th field.
        Fields::Named(ref fields) => {
            let safety_conds: Vec<TokenStream> =
                fields.named.iter().filter_map(|field| parse_safety_expr(ident, field)).collect();
            quote! { #(#safety_conds)&&* }
        }
        Fields::Unnamed(_) => {
            quote! {
                true
            }
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

#[cfg(feature = "no_core")]
fn kani_path() -> TokenStream {
    quote! { core::kani }
}

#[cfg(not(feature = "no_core"))]
fn kani_path() -> TokenStream {
    quote! { kani }
}
