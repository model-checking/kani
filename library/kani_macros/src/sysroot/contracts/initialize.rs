// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Initialization routine for the contract handler

use proc_macro::{Diagnostic, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use syn::ItemFn;

use super::{
    ContractConditionsData, ContractConditionsHandler, ContractConditionsType,
    ContractFunctionState,
    helpers::{chunks_by, is_token_stream_2_comma, matches_path},
};

impl<'a> TryFrom<&'a syn::Attribute> for ContractFunctionState {
    type Error = Option<Diagnostic>;

    /// Find out if this attribute could be describing a "contract handling"
    /// state and if so return it.
    fn try_from(attribute: &'a syn::Attribute) -> Result<Self, Self::Error> {
        if let syn::Meta::NameValue(nv) = &attribute.meta
            && matches_path(&nv.path, &["kanitool", "checked_with"])
        {
            return Ok(ContractFunctionState::Expanded);
        }
        Err(None)
    }
}

impl ContractFunctionState {
    // If we didn't find any other contract handling related attributes we
    // assume this function has not been touched by a contract before.
    pub fn from_attributes(attributes: &[syn::Attribute]) -> Self {
        attributes
            .iter()
            .find_map(|attr| {
                let state = ContractFunctionState::try_from(attr);
                if let Err(Some(diag)) = state {
                    diag.emit();
                    None
                } else {
                    state.ok()
                }
            })
            .unwrap_or(ContractFunctionState::Untouched)
    }
}

impl<'a> ContractConditionsHandler<'a> {
    /// Initialize the handler. Constructs the required
    /// [`ContractConditionsType`] depending on `is_requires`.
    pub fn new(
        is_requires: ContractConditionsType,
        attr: TokenStream,
        annotated_fn: &'a mut ItemFn,
        attr_copy: TokenStream2,
    ) -> Result<Self, syn::Error> {
        let mut output = TokenStream2::new();
        let condition_type = match is_requires {
            ContractConditionsType::Requires => {
                ContractConditionsData::Requires { attr: syn::parse(attr)? }
            }
            ContractConditionsType::Ensures => {
                ContractConditionsData::Ensures { attr: syn::parse(attr)? }
            }
            ContractConditionsType::Modifies => {
                ContractConditionsData::new_modifies(attr, &mut output)
            }
        };

        let fn_name = &annotated_fn.sig.ident;
        let generate_name = |purpose| format!("__kani_{purpose}_{fn_name}");
        let assert_name = generate_name("assert");
        let check_name = generate_name("check");
        let replace_name = generate_name("replace");
        let recursion_name = generate_name("recursion_check");
        let modifies_name = generate_name("modifies");

        Ok(Self {
            condition_type,
            annotated_fn,
            attr_copy,
            output,
            check_name,
            replace_name,
            recursion_name,
            assert_name,
            modify_name: modifies_name,
        })
    }
}
impl ContractConditionsData {
    /// Constructs a [`Self::Modifies`] from the contents of the decorating attribute.
    ///
    /// Responsible for parsing the attribute.
    fn new_modifies(attr: TokenStream, output: &mut TokenStream2) -> Self {
        let attr = chunks_by(TokenStream2::from(attr), is_token_stream_2_comma)
            .map(syn::parse2)
            .filter_map(|expr| match expr {
                Err(e) => {
                    output.extend(e.into_compile_error());
                    None
                }
                Ok(expr) => Some(expr),
            })
            .collect();

        ContractConditionsData::Modifies { attr }
    }
}
