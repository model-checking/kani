// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Logic to implement gen_proof_for_contract macro

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitInt, parse_macro_input, parse_quote};

/// Implementation of [crate::gen_proof_for_contract] when building with `kani_sysroot` enabled.
pub fn gen_proof_for_contract(item: TokenStream) -> TokenStream {
    let ProofForContract { harness_name, fn_path, num_args } = parse_macro_input!(item);
    let kani_any: Expr = parse_quote! { kani::any() };
    let args = (0..num_args).map(|_i| kani_any.clone()).collect::<Vec<_>>();
    quote! {
        #[kani::proof_for_contract(#fn_path)]
        fn  #harness_name() {
            let _ = #fn_path(#(#args),*);
        }
    }
    .into()
}

#[derive(Debug)]
struct ProofForContract {
    harness_name: syn::Ident,
    fn_path: syn::Path,
    num_args: usize,
}

impl Parse for ProofForContract {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let harness_name = input.parse::<syn::Ident>()?;
        let _ = input.parse::<syn::Token![,]>()?;
        let fn_path = input.parse::<syn::Path>()?;
        let _ = input.parse::<syn::Token![,]>()?;
        let num_args = input.parse::<LitInt>()?;
        Ok(ProofForContract { harness_name, fn_path, num_args: num_args.base10_parse()? })
    }
}
