// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::TokenStream;

#[cfg(all(not(kani), not(test)))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Not-Kani, Not-Test means this code shouldn't exist, return nothing.
    TokenStream::new()
}

#[cfg(all(not(kani), test))]
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

#[cfg(kani)]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    result.extend("#[kanitool::proof]".parse::<TokenStream>().unwrap());
    // no_mangle is a temporary hack to make the function "public" so it gets codegen'd
    result.extend("#[no_mangle]".parse::<TokenStream>().unwrap());
    result.extend(item);
    result
    // quote!(
    //     #[kanitool::proof]
    //     $item
    // )
}
