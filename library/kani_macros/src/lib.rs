// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"

mod derive;

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[cfg(kani_sysroot)]
use sysroot as attr_impl;

#[cfg(not(kani_sysroot))]
use regular as attr_impl;

/// Marks a Kani proof harness
///
/// For async harnesses, this will call [`kani::block_on`] (see its documentation for more information).
#[proc_macro_attribute]
pub fn proof(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::proof(attr, item)
}

/// Specifies that a proof harness is expected to panic.**
///
/// This attribute allows users to exercise *negative verification*.
/// It's analogous to how
/// [`#[should_panic]`](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics)
/// allows users to exercise [negative testing](https://en.wikipedia.org/wiki/Negative_testing)
/// for Rust unit tests.
///
/// # Limitations
///
/// The `#[kani::should_panic]` attribute verifies that there are one or more failed checks related to panics.
/// At the moment, it's not possible to pin it down to specific panics.
#[proc_macro_attribute]
pub fn should_panic(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::should_panic(attr, item)
}

/// Set Loop unwind limit for proof harnesses
/// The attribute '#[kani::unwind(arg)]' can only be called alongside '#[kani::proof]'.
/// arg - Takes in a integer value (u32) that represents the unwind value for the harness.
#[proc_macro_attribute]
pub fn unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::unwind(attr, item)
}

/// Specify a function/method stub pair to use for proof harness
///
/// The attribute `#[kani::stub(original, replacement)]` can only be used alongside `#[kani::proof]`.
///
/// # Arguments
/// * `original` - The function or method to replace, specified as a path.
/// * `replacement` - The function or method to use as a replacement, specified as a path.
#[proc_macro_attribute]
pub fn stub(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::stub(attr, item)
}

/// Select the SAT solver to use with CBMC for this harness
/// The attribute `#[kani::solver(arg)]` can only be used alongside `#[kani::proof]``
///
/// arg - name of solver, e.g. kissat
#[proc_macro_attribute]
pub fn solver(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::solver(attr, item)
}

/// Mark an API as unstable. This should only be used inside the Kani sysroot.
/// See https://model-checking.github.io/kani/rfc/rfcs/0006-unstable-api.html for more details.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn unstable(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::unstable(attr, item)
}

/// Allow users to auto generate Arbitrary implementations by using `#[derive(Arbitrary)]` macro.
#[proc_macro_error]
#[proc_macro_derive(Arbitrary)]
pub fn derive_arbitrary(item: TokenStream) -> TokenStream {
    derive::expand_derive_arbitrary(item)
}

#[proc_macro_attribute]
pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::requires(attr, item)
}

#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::ensures(attr, item)
}

/// This module implements Kani attributes in a way that only Kani's compiler can understand.
/// This code should only be activated when pre-building Kani's sysroot.
#[cfg(kani_sysroot)]
mod sysroot {

    use super::*;

    use {
        quote::{format_ident, quote},
        syn::{parse_macro_input, ItemFn},
    };

    use proc_macro2::Ident;

    fn hash_of_token_stream<H: std::hash::Hasher>(
        hasher: &mut H,
        stream: proc_macro2::TokenStream,
    ) {
        use proc_macro2::TokenTree;
        use std::hash::Hash;
        for token in stream {
            match token {
                TokenTree::Ident(i) => i.hash(hasher),
                TokenTree::Punct(p) => p.as_char().hash(hasher),
                TokenTree::Group(g) => {
                    std::mem::discriminant(&g.delimiter()).hash(hasher);
                    hash_of_token_stream(hasher, g.stream());
                }
                TokenTree::Literal(lit) => lit.to_string().hash(hasher),
            }
        }
    }

    /// Annotate the harness with a #[kanitool::<name>] with optional arguments.
    macro_rules! kani_attribute {
        ($name:ident) => {
            pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
                let args = proc_macro2::TokenStream::from(attr);
                let fn_item = parse_macro_input!(item as ItemFn);
                let attribute = format_ident!("{}", stringify!($name));
                quote!(
                    #[kanitool::#attribute(#args)]
                    #fn_item
                ).into()
            }
        };
        ($name:ident, no_args) => {
            pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
                assert!(attr.is_empty(), "`#[kani::{}]` does not take any arguments currently", stringify!($name));
                let fn_item = parse_macro_input!(item as ItemFn);
                let attribute = format_ident!("{}", stringify!($name));
                quote!(
                    #[kanitool::#attribute]
                    #fn_item
                ).into()
            }
        };
    }

    pub fn proof(attr: TokenStream, item: TokenStream) -> TokenStream {
        let fn_item = parse_macro_input!(item as ItemFn);
        let attrs = fn_item.attrs;
        let vis = fn_item.vis;
        let sig = fn_item.sig;
        let body = fn_item.block;

        let kani_attributes = quote!(
            #[allow(dead_code)]
            #[kanitool::proof]
        );

        assert!(attr.is_empty(), "#[kani::proof] does not take any arguments currently");

        if sig.asyncness.is_none() {
            // Adds `#[kanitool::proof]` and other attributes
            quote!(
                #kani_attributes
                #(#attrs)*
                #vis #sig #body
            )
            .into()
        } else {
            // For async functions, it translates to a synchronous function that calls `kani::block_on`.
            // Specifically, it translates
            // ```ignore
            // #[kani::async_proof]
            // #[attribute]
            // pub async fn harness() { ... }
            // ```
            // to
            // ```ignore
            // #[kani::proof]
            // #[attribute]
            // pub fn harness() {
            //   async fn harness() { ... }
            //   kani::block_on(harness())
            // }
            // ```
            assert!(
                sig.inputs.is_empty(),
                "#[kani::proof] cannot be applied to async functions that take inputs for now"
            );
            let mut modified_sig = sig.clone();
            modified_sig.asyncness = None;
            let fn_name = &sig.ident;
            quote!(
                #kani_attributes
                #(#attrs)*
                #vis #modified_sig {
                    #sig #body
                    kani::block_on(#fn_name())
                }
            )
            .into()
        }
    }

    fn generate_identifier(s: &str) -> Ident {
        Ident::new(s, proc_macro2::Span::mixed_site())
    }

    macro_rules! requires_ensures {
        ($name: ident, $append_return:literal) => {
            pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
                use syn::{FnArg, PatType, PatIdent, Pat, Signature, Token, ReturnType, TypeTuple, punctuated::Punctuated};
                use proc_macro2::Span;
                let attr = proc_macro2::TokenStream::from(attr);

                let a_short_hash = {
                    use std::hash::Hasher;
                    let mut hasher = std::collections::hash_map::DefaultHasher::default();
                    hash_of_token_stream(&mut hasher, proc_macro2::TokenStream::from(item.clone()));
                    let long_hash = hasher.finish();
                    long_hash % 0x1_000_000 // six hex digits
                };

                let item_fn @ ItemFn { sig, .. } = &parse_macro_input!(item as ItemFn);
                let Signature { ident, generics, inputs, output , .. } = sig;

                let gen_fn_name = generate_identifier(&format!(concat!("{}_", stringify!($name), "_{:x}"), ident, a_short_hash));
                let attribute = format_ident!("{}", stringify!($name));
                let kani_attributes = quote!(
                    #[allow(dead_code)]
                    #[kanitool::#attribute = stringify!(#gen_fn_name)]
                );

                let typ = match output {
                    ReturnType::Type(_, t) => t.clone(),
                    _ => Box::new(TypeTuple { paren_token: Default::default(), elems: Punctuated::new() }.into()),
                };

                let mut gen_fn_inputs = inputs.clone();
                gen_fn_inputs.insert(
                    0,
                    FnArg::Typed(PatType {
                        attrs: vec![],
                        pat: Box::new(Pat::Ident(PatIdent{
                            attrs: vec![],
                            by_ref: None,
                            mutability: None,
                            ident: Ident::new("result", Span::call_site()),
                            subpat: None,
                        })),
                        colon_token: Token![:](Span::call_site()),
                        ty: typ,
                    })
                );

                assert!(
                    generics.params.is_empty() && generics.where_clause.is_none(),
                    "Generics are not yet implemented",
                );

                quote!(
                    fn #gen_fn_name(#gen_fn_inputs) -> bool {
                        #attr
                    }

                    #kani_attributes
                    #item_fn
                )
                .into()
            }
        }
    }

    requires_ensures!(requires, false);
    requires_ensures!(ensures, true);

    kani_attribute!(should_panic, no_args);
    kani_attribute!(solver);
    kani_attribute!(stub);
    kani_attribute!(unstable);
    kani_attribute!(unwind);
}

/// This module provides dummy implementations of Kani attributes which cannot be interpreted by
/// other tools such as MIRI and the regular rust compiler.
///
/// This allow users to use code marked with Kani attributes, for example, for IDE code inspection.
#[cfg(not(kani_sysroot))]
mod regular {
    use super::*;

    /// Encode a noop proc macro which ignores the given attribute.
    macro_rules! no_op {
        ($name:ident) => {
            pub fn $name(_attr: TokenStream, item: TokenStream) -> TokenStream {
                item
            }
        };
    }

    /// Add #[allow(dead_code)] to a proof harness to avoid dead code warnings.
    pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
        let mut result = TokenStream::new();
        result.extend("#[allow(dead_code)]".parse::<TokenStream>().unwrap());
        result.extend(item);
        result
    }

    no_op!(should_panic);
    no_op!(solver);
    no_op!(stub);
    no_op!(unstable);
    no_op!(unwind);
    no_op!(requires);
    no_op!(ensures);
}
