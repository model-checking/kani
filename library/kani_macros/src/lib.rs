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
#[cfg(kani)]
use {
    quote::quote,
    syn::{parse_macro_input, ItemFn},
};

#[cfg(any(not(kani), feature = "concrete_playback"))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Leave the code intact, so it can be easily be edited in an IDE,
    // but outside Kani, this code is likely never called.
    let mut result = TokenStream::new();

    result.extend("#[allow(dead_code)]".parse::<TokenStream>().unwrap());
    result.extend(item);
    result
    // quote!(
    //     #[allow(dead_code)]
    //     $item
    // )
}

/// Marks a Kani proof harness
///
/// For async harnesses, this will call [`kani::block_on`] (see its documentation for more information).
#[cfg(all(kani, not(feature = "concrete_playback")))]
#[proc_macro_attribute]
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

#[cfg(any(not(kani), feature = "concrete_playback"))]
#[proc_macro_attribute]
pub fn should_panic(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // No-op in non-kani mode
    item
}

#[cfg(all(kani, not(feature = "concrete_playback")))]
#[proc_macro_attribute]
pub fn should_panic(attr: TokenStream, item: TokenStream) -> TokenStream {
    assert!(attr.is_empty(), "`#[kani::should_panic]` does not take any arguments currently");
    let mut result = TokenStream::new();
    let insert_string = "#[kanitool::should_panic]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());

    result.extend(item);
    result
}

#[cfg(any(not(kani), feature = "concrete_playback"))]
#[proc_macro_attribute]
pub fn unwind(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not kani, we should leave the function alone
    item
}

/// Set Loop unwind limit for proof harnesses
/// The attribute '#[kani::unwind(arg)]' can only be called alongside '#[kani::proof]'.
/// arg - Takes in a integer value (u32) that represents the unwind value for the harness.
#[cfg(all(kani, not(feature = "concrete_playback")))]
#[proc_macro_attribute]
pub fn unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    // Translate #[kani::unwind(arg)] to #[kanitool::unwind(arg)]
    let insert_string = "#[kanitool::unwind(".to_owned() + &attr.to_string() + ")]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());

    result.extend(item);
    result
}

#[cfg(any(not(kani), feature = "concrete_playback"))]
#[proc_macro_attribute]
pub fn stub(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not kani, we should leave the function alone
    item
}

/// Specify a function/method stub pair to use for proof harness
///
/// The attribute `#[kani::stub(original, replacement)]` can only be used alongside `#[kani::proof]`.
///
/// # Arguments
/// * `original` - The function or method to replace, specified as a path.
/// * `replacement` - The function or method to use as a replacement, specified as a path.
#[cfg(all(kani, not(feature = "concrete_playback")))]
#[proc_macro_attribute]
pub fn stub(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    // Translate #[kani::stub(original, replacement)] to #[kanitool::stub(original, replacement)]
    let insert_string = "#[kanitool::stub(".to_owned() + &attr.to_string() + ")]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());

    result.extend(item);
    result
}

#[cfg(any(not(kani), feature = "concrete_playback"))]
#[proc_macro_attribute]
pub fn solver(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // No-op in non-kani mode
    item
}

/// Select the SAT solver to use with CBMC for this harness
/// The attribute `#[kani::solver(arg)]` can only be used alongside `#[kani::proof]``
///
/// arg - name of solver, e.g. kissat
#[cfg(all(kani, not(feature = "concrete_playback")))]
#[proc_macro_attribute]
pub fn solver(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();
    // Translate `#[kani::solver(arg)]` to `#[kanitool::solver(arg)]`
    let insert_string = "#[kanitool::solver(".to_owned() + &attr.to_string() + ")]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());

    result.extend(item);
    result
}

/// Allow users to auto generate Arbitrary implementations by using `#[derive(Arbitrary)]` macro.
#[proc_macro_error]
#[proc_macro_derive(Arbitrary)]
pub fn derive_arbitrary(item: TokenStream) -> TokenStream {
    derive::expand_derive_arbitrary(item)
}
