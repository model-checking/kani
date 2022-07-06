// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::{
    Ident,
    Group,
    TokenStream,
    TokenTree,
};


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

/// This proc macro does one of the following. (1) if kani is
/// configured, then it substitutes all occurrences of the proptest
/// crate with a custom kani_proptest crate. (2) otherwise, it keeps
/// the body and pastes it in.
///
/// Implementation of the rewrite is done via a state machine in the
/// .1 position of the fold accumulator. After seeing "proptest"
/// token, it puts the span of this token in the option. If "proptest"
/// is followed by ":" or ";", then a re-write is
/// triggered. Otherwise, it pushes the same token and goes back to
/// the original state until "proptest" is seen again.
#[proc_macro]
pub fn translate_from_proptest(input: TokenStream) -> TokenStream {
    const REWRITE_FROM : &str = "proptest";
    const REWRITE_TO : &str = "kani_proptest";

    fn translate_recursive_helper(input: TokenStream) -> TokenStream {
        input.into_iter()
            .fold(
                (TokenStream::new(), None),
                |(mut acc, maybe_proptest_span), cur|
                if let TokenTree::Ident(ident) = cur {
                    if &ident.to_string() == REWRITE_FROM {
                        (acc, Some(ident.span()))
                    } else {
                        acc.extend(vec![TokenTree::Ident(ident)]);
                        (acc, maybe_proptest_span)
                    }
                } else if let TokenTree::Punct(punctuation) = cur {
                    if let Some(proptest_span) = maybe_proptest_span {
                        acc.extend(vec![
                            TokenTree::Ident(
                                Ident::new_raw(
                                    if punctuation.as_char() == ':' || punctuation.as_char() == ';' {
                                        REWRITE_TO
                                    } else {
                                        REWRITE_FROM
                                    },
                                    proptest_span
                                )
                            ),
                            TokenTree::Punct(punctuation)]
                        );
                        (acc, None)
                    } else {
                        acc.extend(vec![TokenTree::Punct(punctuation)]);
                        (acc, None)
                    }
                } else if let TokenTree::Group(group) = cur {
                    let delimiter = group.delimiter();
                    let stream = translate_recursive_helper(group.stream());
                    acc.extend(vec![TokenTree::Group(Group::new(delimiter, stream))]);
                    (acc, None)
                } else {
                    acc.extend(vec![cur]);
                    (acc, None)
                }
            ).0
    }


    if std::env::var_os("CARGO_CFG_KANI").is_some() {
        let result = translate_recursive_helper(input);
        result
    } else {
        input
    }
}
