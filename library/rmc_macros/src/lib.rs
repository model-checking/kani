// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(rmctool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see rmc-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(rmctool)"

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::TokenStream;

#[cfg(not(rmc))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Leave the code intact, so it can be easily be edited in an IDE,
    // but outside RMC, this code is likely never called.
    let mut result = TokenStream::new();

    result.extend("#[allow(dead_code)]".parse::<TokenStream>().unwrap());
    result.extend(item);
    result
    // quote!(
    //     #[allow(dead_code)]
    //     $item
    // )
}

#[cfg(rmc)]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    result.extend("#[rmctool::proof]".parse::<TokenStream>().unwrap());
    // no_mangle is a temporary hack to make the function "public" so it gets codegen'd
    result.extend("#[no_mangle]".parse::<TokenStream>().unwrap());
    result.extend(item);
    result
    // quote!(
    //     #[rmctool::proof]
    //     $item
    // )
}
