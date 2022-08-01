// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub struct ContractAttributes {
    pub attributes: Vec<syn::Attribute>,
}

impl ContractAttributes {
    pub fn extract_preconditions(&self) -> proc_macro2::TokenStream {
        self.attributes
            .iter()
            .filter_map(|a| {
                let name = a.path.segments.last().unwrap().ident.to_string();
                let arg = a.parse_args::<syn::Expr>().unwrap();
                match name.as_str() {
                    "requires" => Some(quote::quote! { kani::precondition(#arg);}),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn extract_postconditions(&self) -> proc_macro2::TokenStream {
        self.attributes
            .iter()
            .filter_map(|a| {
                let name = a.path.segments.last().unwrap().ident.to_string();
                let arg = a.parse_args::<syn::Expr>().unwrap();
                match name.as_str() {
                    "ensures" => Some(quote::quote! { kani::postcondition(#arg);}),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn extract_write_set(&self) -> proc_macro2::TokenStream {
        self.attributes
            .iter()
            .filter_map(|a| {
                let name = a.path.segments.last().unwrap().ident.to_string();
                let arg = a.parse_args::<syn::Expr>().unwrap();
                match name.as_str() {
                    "assigns" => Some(quote::quote! { kani::write_set(#arg);}),
                    _ => None,
                }
            })
            .collect()
    }
}
