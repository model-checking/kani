// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use proc_macro_error2::abort;
use proc_macro2::{Span, TokenStream};
use quote::quote;

use crate::derive::kani_path;

/// Generate the `DeriveArbitrary` implementation for the given type.
///
/// Fields of the given type marked with `#[bounded]` will use
/// `BoundedArbitrary::bounded_any()` while other fields fall back to `kani::any()`
///
/// Current limitation: Generic bounds are restricted to `T: kani::Arbitrary` rather than
/// `T: kani::BoundedArbitrary`. This is the right thing to do when the generic is
/// used in some other container that you want to be bounded; like in the following
/// example:
///
/// ```rust
/// #[derive(BoundedArbitrary)]
/// struct MyVec<T> {
///     #[bounded]
///     vec: Vec<T>,
///     cap: usize
/// }
/// ```
///
/// However, if you use the generic raw in a field and want it to be bounded, this
/// won't work. The following doesn't compile:
///
/// ```rust
/// #[derive(BoundedArbitrary)]
/// struct Foo<T> {
///     #[bounded]
///     bar: T
/// }
/// ```
///
/// TODO: have the generic bound change based on how it's used if we can detect this
/// automatically, otherwise support an attribute on the generic.
pub(crate) fn expand_derive_bounded_arbitrary(
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let parsed = syn::parse_macro_input!(item as syn::DeriveInput);

    let constructor = match &parsed.data {
        syn::Data::Struct(data_struct) => {
            generate_type_constructor(quote!(Self), &data_struct.fields)
        }
        syn::Data::Enum(data_enum) => enum_constructor(&parsed.ident, data_enum),
        syn::Data::Union(data_union) => union_constructor(&parsed.ident, data_union),
    };

    // add `T: Arbitrary` bounds for generics
    let (generics, clauses) = quote_generics(&parsed.generics);
    let name = &parsed.ident;

    // generate the implementation
    let kani_path = kani_path();
    quote! {
        impl #generics #kani_path::BoundedArbitrary for #name #generics
            #clauses
        {
            fn bounded_any<const N: usize>() -> Self {
                #constructor
            }
        }
    }
    .into()
}

/// Generates the call to construct the given type like so:
///
/// ```
/// Foo(kani::any::<A>(), String::bounded_any::<B, N>())
/// Foo {
///     x: kani::any::<A>(),
///     y: kani::bounded_any::<B, N>()
/// }
/// ```
fn generate_type_constructor(type_name: TokenStream, fields: &syn::Fields) -> TokenStream {
    let field_calls = fields.iter().map(generate_any_call);
    if fields.iter().all(|f| f.ident.is_some()) {
        quote!(#type_name { #(#field_calls),* })
    } else {
        quote!(#type_name( #(#field_calls),* ))
    }
}

/// Generates a `match` case to construct each variant of the given type. Uses a
/// symbolic `usize` to decide which variant to construct.
fn enum_constructor(ident: &syn::Ident, data_enum: &syn::DataEnum) -> TokenStream {
    let variant_constructors = data_enum.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        generate_type_constructor(quote!(#ident::#variant_name), &variant.fields)
    });
    let n_variants = data_enum.variants.len();
    let cases = variant_constructors.enumerate().map(|(idx, var_constr)| {
        if idx < n_variants - 1 { quote!(#idx => #var_constr) } else { quote!(_ => #var_constr) }
    });

    let kani_path = kani_path();
    quote! {
        match #kani_path::any() {
            #(#cases),* ,
        }
    }
}

fn union_constructor(ident: &syn::Ident, _data_union: &syn::DataUnion) -> TokenStream {
    abort!(Span::call_site(), "Cannot derive `BoundedArbitrary` for `{}` union", ident;
           note = ident.span() =>
           "`#[derive(BoundedArbitrary)]` cannot be used for unions such as `{}`", ident
    )
}

/// Generate the necessary generic parameter declarations and generic bounds for a
/// type.
///
/// ```rust
/// impl<A, B> BoundedArbitrary for Foo<A, B>
/// where
///     A: Arbitrary
///     B: Arbitrary
/// {
///     ...
/// }
/// ```
fn quote_generics(generics: &syn::Generics) -> (TokenStream, TokenStream) {
    let kani_path = kani_path();
    let params = generics.type_params().map(|param| quote!(#param)).collect::<Vec<_>>();
    let where_clauses = generics.type_params().map(|param| quote!(#param : #kani_path::Arbitrary));
    if !params.is_empty() {
        (quote!(<#(#params),*>), quote!(where #(#where_clauses),*))
    } else {
        Default::default()
    }
}

/// Generates a symbolic value based on whether the field has the `#[bounded]`
/// attribute. If the field is not bounded, generate `kani::any()` otherwise generate
/// `kani::bounded_any()`.
fn generate_any_call(field: &syn::Field) -> TokenStream {
    let ty = &field.ty;
    let kani_path = kani_path();
    let any_call = if field.attrs.iter().any(|attr| attr.path().is_ident("bounded")) {
        quote!(#kani_path::bounded_any::<#ty, N>())
    } else {
        quote!(#kani_path::any::<#ty>())
    };

    let ident_tok = field.ident.as_ref().map(|ident| quote!(#ident: ));
    quote!(#ident_tok #any_call)
}
