// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Initialization routine for the contract handler

use std::collections::{HashMap, HashSet};

use proc_macro::{Diagnostic, TokenStream};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{spanned::Spanned, visit::Visit, Expr, ExprClosure, ItemFn};

use super::{
    helpers::{chunks_by, is_token_stream_2_comma, matches_path},
    ContractConditionsData, ContractConditionsHandler, ContractConditionsType,
    ContractFunctionState, INTERNAL_RESULT_IDENT,
};

impl<'a> TryFrom<&'a syn::Attribute> for ContractFunctionState {
    type Error = Option<Diagnostic>;

    /// Find out if this attribute could be describing a "contract handling"
    /// state and if so return it.
    fn try_from(attribute: &'a syn::Attribute) -> Result<Self, Self::Error> {
        if let syn::Meta::List(lst) = &attribute.meta {
            if matches_path(&lst.path, &["kanitool", "is_contract_generated"]) {
                let ident = syn::parse2::<Ident>(lst.tokens.clone())
                    .map_err(|e| Some(lst.span().unwrap().error(format!("{e}"))))?;
                let ident_str = ident.to_string();
                return match ident_str.as_str() {
                    "check" => Ok(Self::Check),
                    "replace" => Ok(Self::Replace),
                    "wrapper" => Ok(Self::ModifiesWrapper),
                    _ => {
                        Err(Some(lst.span().unwrap().error("Expected `check` or `replace` ident")))
                    }
                };
            }
        }
        if let syn::Meta::NameValue(nv) = &attribute.meta {
            if matches_path(&nv.path, &["kanitool", "checked_with"]) {
                return Ok(ContractFunctionState::Original);
            }
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
        function_state: ContractFunctionState,
        is_requires: ContractConditionsType,
        attr: TokenStream,
        annotated_fn: &'a mut ItemFn,
        attr_copy: TokenStream2,
        hash: Option<u64>,
    ) -> Result<Self, syn::Error> {
        let mut output = TokenStream2::new();
        let condition_type = match is_requires {
            ContractConditionsType::Requires => {
                ContractConditionsData::Requires { attr: syn::parse(attr)? }
            }
            ContractConditionsType::Ensures => {
                let data: ExprClosure = syn::parse(attr)?;
                let argument_names = rename_argument_occurrences(&annotated_fn.sig);
                let result: Ident = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
                let app: Expr = Expr::Verbatim(quote!((#data)(&#result)));
                ContractConditionsData::Ensures { argument_names, attr: app }
            }
            ContractConditionsType::Modifies => {
                ContractConditionsData::new_modifies(attr, &mut output)
            }
        };

        Ok(Self { function_state, condition_type, annotated_fn, attr_copy, output, hash })
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

/// A supporting function for creating shallow, unsafe copies of the arguments
/// for the postconditions.
///
/// This function:
/// - Collects all [`Ident`]s found in the argument patterns;
/// - Creates new names for them;
/// - Returns the mapping of old names to new names.
fn rename_argument_occurrences(sig: &syn::Signature) -> HashMap<Ident, Ident> {
    let mut arg_ident_collector = ArgumentIdentCollector::new();
    arg_ident_collector.visit_signature(&sig);

    let mk_new_ident_for = |id: &Ident| Ident::new(&format!("{}_renamed", id), Span::mixed_site());
    let arg_idents = arg_ident_collector
        .0
        .into_iter()
        .map(|i| {
            let new = mk_new_ident_for(&i);
            (i, new)
        })
        .collect::<HashMap<_, _>>();

    arg_idents
}

/// Collect all named identifiers used in the argument patterns of a function.
struct ArgumentIdentCollector(HashSet<Ident>);

impl ArgumentIdentCollector {
    fn new() -> Self {
        Self(HashSet::new())
    }
}

impl<'ast> Visit<'ast> for ArgumentIdentCollector {
    fn visit_pat_ident(&mut self, i: &'ast syn::PatIdent) {
        self.0.insert(i.ident.clone());
        syn::visit::visit_pat_ident(self, i)
    }
}
