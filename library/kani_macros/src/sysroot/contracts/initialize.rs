// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Initialization routine for the contract handler

use std::collections::{HashMap, HashSet};

use proc_macro::{Diagnostic, TokenStream};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use syn::{
    spanned::Spanned, visit::Visit, visit_mut::VisitMut, Expr, ExprCall, ExprPath, ItemFn, Path,
    PathSegment, Signature,
};

use super::{
    helpers::{chunks_by, is_token_stream_2_comma, matches_path},
    ContractConditionsData, ContractConditionsHandler, ContractConditionsType,
    ContractFunctionState,
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
                ContractConditionsData::new_ensures(&annotated_fn.sig, syn::parse(attr)?)
            }
            ContractConditionsType::Modifies => {
                ContractConditionsData::new_modifies(attr, &mut output)
            }
        };

        Ok(Self { function_state, condition_type, annotated_fn, attr_copy, output, hash })
    }
}
impl ContractConditionsData {
    /// Constructs a [`Self::Ensures`] from the signature of the decorated
    /// function and the contents of the decorating attribute.
    ///
    /// Renames the [`Ident`]s used in `attr` and stores the translation map in
    /// `argument_names`.
    fn new_ensures(sig: &Signature, mut attr: Expr) -> Self {
        let old_replacer = {
            let mut vis = OldVisitor::new(OldLifter::new());
            vis.visit_expr_mut(&mut attr);
            vis.into_inner()
        };
        // Make sure we use unique names here
        let history_expressions = old_replacer.into_iter_exprs_and_idents().collect();
        let argument_names = rename_argument_occurrences(sig, &mut attr);
        ContractConditionsData::Ensures { argument_names, attr, history_expressions }
    }

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
/// - Replaces all occurrences of those idents in `attrs` with the new names and;
/// - Returns the mapping of old names to new names.
fn rename_argument_occurrences(sig: &syn::Signature, attr: &mut Expr) -> HashMap<Ident, Ident> {
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

    let mut ident_rewriter = Renamer(&arg_idents);
    ident_rewriter.visit_expr_mut(attr);
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
    fn visit_receiver(&mut self, _: &'ast syn::Receiver) {
        self.0.insert(Ident::new("self", proc_macro2::Span::call_site()));
    }
}

/// Applies the contained renaming (key renamed to value) to every ident pattern
/// and ident expr visited.
struct Renamer<'a>(&'a HashMap<Ident, Ident>);

impl<'a> VisitMut for Renamer<'a> {
    fn visit_expr_path_mut(&mut self, i: &mut syn::ExprPath) {
        if i.path.segments.len() == 1 {
            i.path
                .segments
                .first_mut()
                .and_then(|p| self.0.get(&p.ident).map(|new| p.ident = new.clone()));
        }
    }

    /// This restores shadowing. Without this we would rename all ident
    /// occurrences, but not rebinding location. This is because our
    /// [`Self::visit_expr_path_mut`] is scope-unaware.
    fn visit_pat_ident_mut(&mut self, i: &mut syn::PatIdent) {
        if let Some(new) = self.0.get(&i.ident) {
            i.ident = new.clone();
        }
    }
}

trait OldTrigger {
    /// You are provided the expression that is the first argument of the
    /// `old()` call. You may modify it as you see fit. The return value
    /// indicates whether the entire `old()` call should be replaced by the
    /// (potentially altered) first argument.
    ///
    /// The second argument is the span of the original `old` expr
    fn trigger(&mut self, e: &mut Expr, s: Span) -> bool;
}

struct OldLifter(Vec<Expr>);

impl OldLifter {
    fn generate_identifier_for(index: usize) -> Ident {
        let gen_ident = format!("old_{index}");
        Ident::new(&gen_ident, proc_macro2::Span::mixed_site())
    }

    fn into_iter_exprs_and_idents(self) -> impl Iterator<Item = (Ident, Expr)> {
        self.0
            .into_iter()
            .enumerate()
            .map(|(index, e)| (OldLifter::generate_identifier_for(index), e))
    }

    fn new() -> Self {
        Self(vec![])
    }
}

struct OldDenier;

impl OldTrigger for OldDenier {
    fn trigger(&mut self, _: &mut Expr, s: Span) -> bool {
        s.unwrap().error("Nested calls to `old` are prohibited, because they are not well defined (what would it even mean?)").emit();
        false
    }
}

struct OldVisitor<T>(T);

impl<T: OldTrigger> OldVisitor<T> {
    fn new(t: T) -> Self {
        Self(t)
    }

    fn into_inner(self) -> T {
        self.0
    }
}

impl<T: OldTrigger> syn::visit_mut::VisitMut for OldVisitor<T> {
    fn visit_expr_mut(&mut self, ex: &mut Expr) {
        let trigger = match &*ex {
            Expr::Call(call @ ExprCall { func, attrs, args, .. }) => match func.as_ref() {
                Expr::Path(ExprPath {
                    attrs: func_attrs,
                    qself: None,
                    path: Path { leading_colon: None, segments },
                }) if segments.len() == 1
                    && segments.first().map_or(false, |sgm| sgm.ident == "old") =>
                {
                    let first_segment = segments.first().unwrap();
                    assert_spanned_err!(first_segment.arguments.is_empty(), first_segment);
                    assert_spanned_err!(attrs.is_empty(), call);
                    assert_spanned_err!(func_attrs.is_empty(), func);
                    assert_spanned_err!(args.len() == 1, call);
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if trigger {
            let span = ex.span();
            let new_expr = if let Expr::Call(ExprCall { ref mut args, .. }) = ex {
                self.0
                    .trigger(args.iter_mut().next().unwrap(), span)
                    .then(|| args.pop().unwrap().into_value())
            } else {
                unreachable!()
            };
            if let Some(new) = new_expr {
                let _ = std::mem::replace(ex, new);
            }
        } else {
            syn::visit_mut::visit_expr_mut(self, ex)
        }
    }
}

impl OldTrigger for OldLifter {
    fn trigger(&mut self, e: &mut Expr, _: Span) -> bool {
        let mut denier = OldVisitor::new(OldDenier);
        // This ensures there are no nested calls to `old`
        denier.visit_expr_mut(e);

        self.0.push(std::mem::replace(
            e,
            Expr::Path(ExprPath {
                attrs: vec![],
                qself: None,
                path: Path {
                    leading_colon: None,
                    segments: [PathSegment {
                        ident: OldLifter::generate_identifier_for(self.0.len()),
                        arguments: syn::PathArguments::None,
                    }]
                    .into_iter()
                    .collect(),
                },
            }),
        ));
        true
    }
}

struct IdentToOldRewriter;

impl syn::visit_mut::VisitMut for IdentToOldRewriter {
    fn visit_pat_ident_mut(&mut self, i: &mut syn::PatIdent) {
        i.ident = Ident::new(&format!("old_{}", i.ident.to_string()), i.span())
    }
}
