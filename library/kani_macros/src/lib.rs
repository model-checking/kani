// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
mod contract;
use syn::{parse_macro_input, punctuated::Punctuated, Expr, ItemFn, Token};

#[cfg(all(not(kani), not(test)))]
#[proc_macro_attribute]
pub fn proof(_attr: TokenStream, _item: TokenStream) -> TokenStream {
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
pub fn proof(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    assert!(attr.to_string().len() == 0, "#[kani::proof] does not take any arguments");
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

#[cfg(not(kani))]
#[proc_macro_attribute]
pub fn unwind(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not kani, we should leave the function alone
    item
}

/// Set Loop unwind limit for proof harnesses
/// The attribute '#[kani::unwind(arg)]' can only be called alongside '#[kani::proof]'.
/// arg - Takes in a integer value (u32) that represents the unwind value for the harness.
#[cfg(kani)]
#[proc_macro_attribute]
pub fn unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut result = TokenStream::new();

    // Translate #[kani::unwind(arg)] to #[kanitool::unwind(arg)]
    let insert_string = "#[kanitool::unwind(".to_owned() + &attr.clone().to_string() + ")]";
    result.extend(insert_string.parse::<TokenStream>().unwrap());

    result.extend(item);
    result
}

#[cfg(not(kani))]
#[proc_macro_attribute]
pub fn ensures(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not Kani, we should leave the function alone
    item
}

#[cfg(kani)]
#[proc_macro_attribute]
/// If config is Kani, `#[kani::ensures(arg)]` specifies a postcondition on the function.
/// The postcondition is treated as part of the function's "contract" specification.
/// arg - Takes in a boolean expression that represents the precondition.
/// The following transformations take place during macro expansion:
/// 1) All `#[kani::requires(arg)]` attributes gets translated to `kani::precondition(arg)` and
///     gets injected to the body of the function right before the actual body begins.
/// 2) All `#[kani::ensures(arg)]` attributes gets translated to `kani::postcondition(arg)` and
///     gets injected to the body of the function, after the actual body begins.
/// 3) The body of the original function (say function `foo`) gets wrapped into a closure
///     with name `foo_<uuid>(...)` and the closure is subsequently called.
///     This is done to handle the return value of the original function
///     as `kani:postcondition(..)` gets injected after the body of the function.
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_attr = parse_macro_input!(attr as Expr);

    // Extract other contract clauses from "item"
    let mut parsed_item = parse_macro_input!(item as ItemFn);
    let other_attributes = parsed_item.attrs.clone();
    let non_inlined = contract::extract_non_inlined_attributes(&other_attributes);
    let pre = contract::extract_requires_as_preconditions(&other_attributes);
    let post = contract::extract_ensures_as_postconditions(&other_attributes);

    // Extract components of the function from "item"
    let fn_vis = parsed_item.vis.clone();
    let fn_sig = parsed_item.sig.clone();
    let args = contract::extract_function_args(&fn_sig);

    // Wrap original function body in a closure
    let (closure_ident, fn_closure) = contract::convert_to_closure(&parsed_item);
    quote::quote! {
        #non_inlined
        #fn_vis #fn_sig {
            #pre
            #fn_closure
            let ret = if kani::replace_function_body() {
                kani::any()
            } else {
                #closure_ident(#(#args,)*)
            };
            kani::postcondition(#parsed_attr);
            #post
            ret
        }
    }
    .into()
}

#[cfg(not(kani))]
#[proc_macro_attribute]
pub fn requires(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not Kani, we should leave the function alone
    item
}

#[cfg(kani)]
#[proc_macro_attribute]
/// If config is Kani, `#[kani::requires(arg)]` adds a precondition on the function.
/// The precondition is treated as part of the function's "contract" specification.
/// The following transformations take place during macro expansion:
/// 1) All `#[kani::requires(arg)]` attributes gets translated to `kani::precondition(arg)` and
///     gets injected to the body of the function right before the actual body begins.
/// 2) All `#[kani::ensures(arg)]` attributes gets translated to `kani::postcondition(arg)` and
///     gets injected to the body of the function, after the actual body begins.
/// 3) The body of the original function (say function `foo`) gets wrapped into a closure
///     with name `foo_<uuid>(...)` and the closure is subsequently called.
///     This is done to handle the return value of the original function
///     as `kani:postcondition(..)` gets injected after the body of the function.
pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    let parsed_attr = parse_macro_input!(attr as Expr);

    // Extract other contract clauses from "item"
    let mut parsed_item = parse_macro_input!(item as ItemFn);
    let other_attributes = parsed_item.attrs.clone();
    let non_inlined = contract::extract_non_inlined_attributes(&other_attributes);
    let pre = contract::extract_requires_as_preconditions(&other_attributes);
    let post = contract::extract_ensures_as_postconditions(&other_attributes);

    // Extract components of the function from "item"
    let fn_vis = parsed_item.vis.clone();
    let fn_sig = parsed_item.sig.clone();
    let args = contract::extract_function_args(&fn_sig);

    // Wrap original function body in a closure
    let (closure_ident, fn_closure) = contract::convert_to_closure(&parsed_item);
    quote::quote! {
        #non_inlined
        #fn_vis #fn_sig {
            kani::precondition(#parsed_attr);
            #pre
            #fn_closure
            let ret = if kani::replace_function_body() {
                kani::any()
            } else {
                #closure_ident(#(#args,)*)
            };
            #post
            ret
        }
    }
    .into()
}

#[cfg(not(kani))]
#[proc_macro_attribute]
pub fn modifies(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // When the config is not Kani, we should leave the function alone
    item
}

#[cfg(kani)]
#[proc_macro_attribute]
/// The attribute '#[kani::modifies(arg1, arg2, ...)]' defines the write set of the function.
/// arg - Zero or more comma-separated “targets” which can be variables.
pub fn modifies(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse 'arg'
    let mut targets: Vec<Expr> =
        parse_macro_input!(attr with Punctuated::<Expr, Token![,]>::parse_terminated)
            .into_iter()
            .collect();

    // Parse 'item' and extract out the function and the remaining attributes.
    let mut parsed_item = parse_macro_input!(item as ItemFn);

    // Extract other modifies clauses from "item"
    let other_attributes = parsed_item.attrs.clone();

    other_attributes.iter().enumerate().for_each(|(i, a)| {
        let name = a.path.segments.last().unwrap().ident.to_string();
        match name.as_str() {
            "modifies" => {
                // Remove from parsed_item.
                parsed_item.attrs.remove(i);
                // Add arguments to list of targets.
                let new_targets: Punctuated<Expr, Token![,]> =
                    a.parse_args_with(Punctuated::parse_terminated).unwrap();
                let new_targets_vec: Vec<Expr> = new_targets.into_iter().collect();
                targets.extend(new_targets_vec);
            }
            _ => {}
        }
    });

    quote::quote! {
        #[kanitool::modifies(#(#targets,)*)]
        #parsed_item
    }
    .into()
}
