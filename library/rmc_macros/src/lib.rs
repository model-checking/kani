// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(rmctool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see rmc-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(rmctool)"

// proc_macro::quote is nightly-only, so we'll cobble things together instead
extern crate proc_macro;
use proc_macro::TokenStream;

#[cfg(all(not(rmc), not(test)))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Not-RMC, Not-Test means this code shouldn't exist, return nothing.
    TokenStream::new()
}

#[cfg(all(not(rmc), test))]
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

#[cfg(not(rmc))]
#[proc_macro_attribute]
pub fn unwind_loop(_attr: TokenStream, _item: TokenStream) -> TokenStream {
    // Not-RMC, Not-Test means this code shouldn't exist, return nothing.
    TokenStream::new()
}

#[cfg(rmc)]
#[proc_macro_attribute]
pub fn unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    // Debug attribute arguments (Metadata of the macro). For ex - #[rmc::unwind(9)] has the metadata = "9"
    let attr_copy = attr.clone().to_string();
    println!("Attributes {}", attr_copy);

    // Translate #[rmc::unwind(arg)] to #[rmctool::unwind_arg_] for easier handling
    let insert_string = "#[rmctool::unwind_".to_owned() + &attr.clone().to_string() + "_]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());
    result.extend("#[no_mangle]".parse::<TokenStream>().unwrap());
    result.extend(item);
    result
    // / _attr
}
