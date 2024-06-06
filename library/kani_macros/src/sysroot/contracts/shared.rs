// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic that is shared between [`super::initialize`], [`super::check`] and
//! [`super::replace`].
//!
//! This is so we can keep [`super`] distraction-free as the definitions of data
//! structures and the entry point for contract handling.

use std::collections::HashMap;

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{Attribute, Expr, ExprClosure, Local, PatIdent, Stmt, ExprCall, ExprPath,
    spanned::Spanned, visit_mut::VisitMut, Path,
    };

use super::{ContractConditionsHandler, ContractFunctionState, INTERNAL_RESULT_IDENT};

impl ContractFunctionState {
    /// Do we need to emit the `is_contract_generated` tag attribute on the
    /// generated function(s)?
    pub fn emit_tag_attr(self) -> bool {
        matches!(self, ContractFunctionState::Untouched)
    }
}

impl<'a> ContractConditionsHandler<'a> {
    pub fn is_first_emit(&self) -> bool {
        matches!(self.function_state, ContractFunctionState::Untouched)
    }

    /// Create a new name for the assigns wrapper function *or* get the name of
    /// the wrapper we must have already generated. This is so that we can
    /// recognize a call to that wrapper inside the check function.
    pub fn make_wrapper_name(&self) -> Ident {
        if let Some(hash) = self.hash {
            identifier_for_generated_function(&self.annotated_fn.sig.ident, "wrapper", hash)
        } else {
            let str_name = self.annotated_fn.sig.ident.to_string();
            let splits = str_name.rsplitn(3, '_').collect::<Vec<_>>();
            let [hash, _, base] = splits.as_slice() else {
                unreachable!("Odd name for function {str_name}, splits were {}", splits.len());
            };

            Ident::new(&format!("{base}_wrapper_{hash}"), Span::call_site())
        }
    }

    /// Emit attributes common to check or replace function into the output
    /// stream.
    pub fn emit_common_header(&mut self) {
        if self.function_state.emit_tag_attr() {
            self.output.extend(quote!(
                #[allow(dead_code, unused_variables)]
            ));
        }
        self.output.extend(self.annotated_fn.attrs.iter().flat_map(Attribute::to_token_stream));
    }
}

/// Makes consistent names for a generated function which was created for
/// `purpose`, from an attribute that decorates `related_function` with the
/// hash `hash`.
pub fn identifier_for_generated_function(
    related_function_name: &Ident,
    purpose: &str,
    hash: u64,
) -> Ident {
    let identifier = format!("{}_{purpose}_{hash:x}", related_function_name);
    Ident::new(&identifier, proc_macro2::Span::mixed_site())
}

/// We make shallow copies of the argument for the postconditions in both
/// `requires` and `ensures` clauses and later clean them up.
///
/// This function creates the code necessary to both make the copies (first
/// tuple elem) and to clean them (second tuple elem).
pub fn make_unsafe_argument_copies(
    renaming_map: &HashMap<Ident, Ident>,
) -> (TokenStream2, TokenStream2) {
    let arg_names = renaming_map.values();
    let also_arg_names = renaming_map.values();
    let arg_values = renaming_map.keys();
    (
        quote!(#(let #arg_names = kani::internal::untracked_deref(&#arg_values);)*),
        quote!(#(std::mem::forget(#also_arg_names);)*),
    )
}

/// Used as the "single source of truth" for [`try_as_result_assign`] and [`try_as_result_assign_mut`]
/// since we can't abstract over mutability. Input is the object to match on and the name of the
/// function used to convert an `Option<LocalInit>` into the result type (e.g. `as_ref` and `as_mut`
/// respectively).
///
/// We start with a `match` as a top-level here, since if we made this a pattern macro (the "clean"
/// thing to do) then we cant use the `if` inside there which we need because box patterns are
/// unstable.
macro_rules! try_as_result_assign_pat {
    ($input:expr, $convert:ident) => {
        match $input {
            syn::Stmt::Local(syn::Local {
                pat: syn::Pat::Type(syn::PatType {
                    pat: inner_pat,
                    attrs,
                    ..
                }),
                init,
                ..
            }) if attrs.is_empty()
            && matches!(
                inner_pat.as_ref(),
                syn::Pat::Ident(syn::PatIdent {
                    by_ref: None,
                    mutability: None,
                    ident: result_ident,
                    subpat: None,
                    ..
                }) if result_ident == INTERNAL_RESULT_IDENT
            ) => init.$convert(),
            _ => None,
        }
    };
}

/// Try to parse this statement as `let result : <...> = <init>;` and return `init`.
///
/// This is the shape of statement we create in replace functions to havoc (with `init` being
/// `kani::any()`) and we need to recognize it for when we edit the replace function and integrate
/// additional conditions.
///
/// It's a thin wrapper around [`try_as_result_assign_pat!`] to create an immutable match.
pub fn try_as_result_assign(stmt: &syn::Stmt) -> Option<&syn::LocalInit> {
    try_as_result_assign_pat!(stmt, as_ref)
}

/// Try to parse this statement as `let result : <...> = <init>;` and return a mutable reference to
/// `init`.
///
/// This is the shape of statement we create in check functions (with `init` being a call to check
/// function with additional pointer arguments for the `modifies` clause) and we need to recognize
/// it to then edit this call if we find another `modifies` clause and add its additional arguments.
/// additional conditions.
///
/// It's a thin wrapper around [`try_as_result_assign_pat!`] to create a mutable match.
pub fn try_as_result_assign_mut(stmt: &mut syn::Stmt) -> Option<&mut syn::LocalInit> {
    try_as_result_assign_pat!(stmt, as_mut)
}

pub fn count_remembers(stmt_vec: &Vec<syn::Stmt>) -> usize {
    stmt_vec
        .iter()
        .filter(|&s: &&syn::Stmt| match s {
            Stmt::Local(Local { attrs: _, let_token: _, pat, init: _, semi_token: _ }) => match pat
            {
                syn::Pat::Ident(PatIdent {
                    attrs: _,
                    by_ref: _,
                    mutability: _,
                    ident,
                    subpat: _,
                }) => ident.to_string().starts_with("remember_kani_internal_"),
                _ => false,
            },
            _ => false,
        })
        .count()
}

pub fn build_ensures(data: &ExprClosure, remember_count: usize) -> (TokenStream2, Expr) {
    let mut remembers_exprs = vec![];
    let mut vis = OldVisitor{t:OldLifter::new(), remember_count, remembers_exprs: &mut remembers_exprs};
    let mut expr = &mut data.clone();
    vis.visit_expr_closure_mut(&mut expr);

    let remembers_stmts : TokenStream2 = remembers_exprs.iter().enumerate().fold(quote!(), |collect, (index, expr)| {
        let rem = index + remember_count;
        let ident = Ident::new(&("remember_kani_internal_".to_owned() + &rem.to_string()), Span::call_site());
        quote!(let #ident = &#expr; #collect)
    });

    let result: Ident = Ident::new(INTERNAL_RESULT_IDENT, Span::call_site());
    (remembers_stmts, Expr::Verbatim(quote!((#expr)(&#result))))
}

trait OldTrigger {
    /// You are provided the expression that is the first argument of the
    /// `old()` call. You may modify it as you see fit. The return value
    /// indicates whether the entire `old()` call should be replaced by the
    /// (potentially altered) first argument.
    ///
    /// The second argument is the span of the original `old` expr
    fn trigger(&mut self, e: &mut Expr, s: Span, remember_count : usize, output : &mut Vec<Expr>) -> bool;
}

struct OldLifter;

impl OldLifter {
    fn new() -> Self {
        Self
    }
}

struct OldDenier;

impl OldTrigger for OldDenier {
    fn trigger(&mut self, _: &mut Expr, s: Span, _ : usize, _ : &mut Vec<Expr>) -> bool {
        s.unwrap().error("Nested calls to `old` are prohibited").emit();
        false
    }
}

struct OldVisitor<'a, T>{
    t : T,
    remember_count : usize,
    remembers_exprs : &'a mut Vec<Expr>,
}

impl<T: OldTrigger> syn::visit_mut::VisitMut for OldVisitor<'_, T> {
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
                self.t
                    .trigger(args.iter_mut().next().unwrap(), span, self.remember_count, self.remembers_exprs)
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
    fn trigger(&mut self, e: &mut Expr, _: Span, remember_count : usize, remembers_exprs : &mut Vec<Expr>) -> bool {
        let mut denier = OldVisitor{t:OldDenier, remember_count, remembers_exprs};
        // This ensures there are no nested calls to `old`
        denier.visit_expr_mut(e);

        let index = remember_count + remembers_exprs.len();
        remembers_exprs.push((*e).clone());
        let ident = Ident::new(&("remember_kani_internal_".to_owned() + &index.to_string()), Span::call_site());
        let _ = std::mem::replace(e,Expr::Verbatim(quote!((*#ident))));
        true
    }
}