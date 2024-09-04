// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Functions that operate third party data structures with no logic that is
//! specific to Kani and contracts.

use crate::attr_impl::contracts::ClosureType;
use proc_macro2::{Ident, Span};
use std::borrow::Cow;
use syn::spanned::Spanned;
use syn::{
    parse_quote, Attribute, Expr, ExprBlock, ExprCall, ExprPath, Local, LocalInit, PatIdent, Path,
    Stmt,
};

/// If an explicit return type was provided it is returned, otherwise `()`.
pub fn return_type_to_type(return_type: &syn::ReturnType) -> Cow<syn::Type> {
    match return_type {
        syn::ReturnType::Default => Cow::Owned(syn::Type::Tuple(syn::TypeTuple {
            paren_token: syn::token::Paren::default(),
            elems: Default::default(),
        })),
        syn::ReturnType::Type(_, typ) => Cow::Borrowed(typ.as_ref()),
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum MutBinding {
    Mut,
    NotMut,
}

/// Extract all local bindings from a given pattern.
///
/// Does not support range patterns, or verbatim.
pub fn pat_to_bindings(pat: &syn::Pat) -> Vec<(MutBinding, &Ident)> {
    use syn::Pat;
    let mk_err = |typ| {
        pat.span()
            .unwrap()
            .error(format!("`{typ}` patterns are not supported for functions with contracts"))
            .emit();
        unreachable!()
    };
    match pat {
        Pat::Const(_) => vec![],
        Pat::Ident(PatIdent { ident, subpat: Some(subpat), mutability, .. }) => {
            let mut idents = pat_to_bindings(subpat.1.as_ref());
            idents.push((mutability.map_or(MutBinding::NotMut, |_| MutBinding::Mut), ident));
            idents
        }
        Pat::Ident(PatIdent { ident, mutability, .. }) => {
            vec![(mutability.map_or(MutBinding::NotMut, |_| MutBinding::Mut), ident)]
        }
        Pat::Lit(_) => vec![],
        Pat::Reference(_) => vec![],
        Pat::Tuple(tup) => tup.elems.iter().flat_map(pat_to_bindings).collect(),
        Pat::Slice(slice) => slice.elems.iter().flat_map(pat_to_bindings).collect(),
        Pat::Path(_) => {
            vec![]
        }
        Pat::Or(pat_or) => {
            // Note: Patterns are not accepted in function arguments.
            // No matter what, the same bindings must exist in all the patterns.
            pat_or.cases.first().map(pat_to_bindings).unwrap_or_default()
        }
        Pat::Rest(_) => vec![],
        Pat::Wild(_) => vec![],
        Pat::Paren(inner) => pat_to_bindings(&inner.pat),
        Pat::Range(_) => vec![],
        Pat::Struct(strct) => {
            strct.fields.iter().flat_map(|field_pat| pat_to_bindings(&field_pat.pat)).collect()
        }
        Pat::Verbatim(_) => mk_err("verbatim"),
        Pat::Type(pt) => pat_to_bindings(pt.pat.as_ref()),
        Pat::TupleStruct(tup) => tup.elems.iter().flat_map(pat_to_bindings).collect(),
        _ => mk_err("unknown"),
    }
}

/// Find a closure statement attached with `kanitool::is_contract_generated` attribute.
pub fn find_contract_closure<'a>(
    stmts: &'a mut [Stmt],
    name: &'static str,
) -> Option<&'a mut Stmt> {
    stmts.iter_mut().find(|stmt| {
        if let Stmt::Local(local) = stmt {
            let ident = Ident::new(name, Span::call_site());
            let attr: Attribute = parse_quote!(#[kanitool::is_contract_generated(#ident)]);
            local.attrs.contains(&attr)
        } else {
            false
        }
    })
}

/// Find a closure defined in one of the provided statements.
///
/// Panic if no closure was found.
pub fn expect_closure<'a>(stmts: &'a mut [Stmt], name: &'static str) -> &'a mut Stmt {
    find_contract_closure(stmts, name)
        .expect(&format!("Internal Failure: Expected to find `{name}` closure, but found none"))
}

/// Find a closure inside a match block.
///
/// Panic if no closure was found.
pub fn expect_closure_in_match<'a>(stmts: &'a mut [Stmt], name: &'static str) -> &'a mut Stmt {
    let closure = stmts.iter_mut().find_map(|stmt| {
        if let Stmt::Expr(Expr::Match(match_expr), ..) = stmt {
            match_expr.arms.iter_mut().find_map(|arm| {
                let Expr::Block(block) = arm.body.as_mut() else { return None };
                find_contract_closure(&mut block.block.stmts, name)
            })
        } else {
            None
        }
    });
    closure.expect(&format!("Internal Failure: Expected to find `{name}` closure, but found none"))
}

/// Extract the body of a closure declaration.
pub fn closure_body(closure: &mut Stmt) -> &mut ExprBlock {
    let Stmt::Local(Local { init: Some(LocalInit { expr, .. }), .. }) = closure else {
        unreachable!()
    };
    let Expr::Closure(closure) = expr.as_mut() else { unreachable!() };
    let Expr::Block(body) = closure.body.as_mut() else { unreachable!() };
    body
}

/// Does the provided path have the same chain of identifiers as `mtch` (match)
/// and no arguments anywhere?
///
/// So for instance (using some pseudo-syntax for the [`syn::Path`]s)
/// `matches_path(std::vec::Vec, &["std", "vec", "Vec"]) == true` but
/// `matches_path(std::Vec::<bool>::contains, &["std", "Vec", "contains"]) !=
/// true`.
///
/// This is intended to be used to match the internal `kanitool` family of
/// attributes which we know to have a regular structure and no arguments.
pub fn matches_path<E>(path: &syn::Path, mtch: &[E]) -> bool
where
    Ident: std::cmp::PartialEq<E>,
{
    path.segments.len() == mtch.len()
        && path.segments.iter().all(|s| s.arguments.is_empty())
        && path.leading_colon.is_none()
        && path.segments.iter().zip(mtch).all(|(actual, expected)| actual.ident == *expected)
}

pub fn is_token_stream_2_comma(t: &proc_macro2::TokenTree) -> bool {
    matches!(t, proc_macro2::TokenTree::Punct(p) if p.as_char() == ',')
}

pub fn chunks_by<'a, T, C: Default + Extend<T>>(
    i: impl IntoIterator<Item = T> + 'a,
    mut pred: impl FnMut(&T) -> bool + 'a,
) -> impl Iterator<Item = C> + 'a {
    let mut iter = i.into_iter();
    std::iter::from_fn(move || {
        let mut new = C::default();
        let mut empty = true;
        for tok in iter.by_ref() {
            empty = false;
            if pred(&tok) {
                break;
            } else {
                new.extend([tok])
            }
        }
        (!empty).then_some(new)
    })
}

/// Splits `stmts` into (preconditions, rest).
/// For example, ClosureType::Check assumes preconditions, so given this sequence of statements:
/// ```ignore
/// kani::assume(.. precondition_1);
/// kani::assume(.. precondition_2);
/// let _wrapper_arg = (ptr as * const _,);
/// ...
/// ```
/// This function would return the two kani::assume statements in the former slice
/// and the remaining statements in the latter.
/// The flow for ClosureType::Replace is the same, except preconditions are asserted rather than assumed.
///
/// The caller can use the returned tuple to insert remembers statements after `preconditions` and before `rest`.
/// Inserting the remembers statements after `preconditions` ensures that they are bound by the preconditions.
/// To understand why this is important, take the following example:
/// ```ignore
/// #[kani::requires(x < u32::MAX)]
/// #[kani::ensures(|result| old(x + 1) == *result)]
/// fn add_one(x: u32) -> u32 {...}
/// ```
/// If the `old(x + 1)` statement didn't respect the precondition's upper bound on `x`, Kani would encounter an integer overflow.
///
/// Inserting the remembers statements before `rest` ensures that they are declared before the original function executes,
/// so that they will store historical, pre-computation values as intended.
pub fn split_for_remembers(stmts: &[Stmt], closure_type: ClosureType) -> (&[Stmt], &[Stmt]) {
    let mut pos = 0;

    let check_str = match closure_type {
        ClosureType::Check => "assume",
        ClosureType::Replace => "assert",
    };

    for stmt in stmts {
        if let Stmt::Expr(Expr::Call(ExprCall { func, .. }), _) = stmt {
            if let Expr::Path(ExprPath { path: Path { segments, .. }, .. }) = func.as_ref() {
                let first_two_idents =
                    segments.iter().take(2).map(|sgmt| sgmt.ident.to_string()).collect::<Vec<_>>();

                if first_two_idents == vec!["kani", check_str] {
                    pos += 1;
                }
            }
        }
    }
    stmts.split_at(pos)
}

macro_rules! assert_spanned_err {
    ($condition:expr, $span_source:expr, $msg:expr, $($args:expr),+) => {
        if !$condition {
            $span_source.span().unwrap().error(format!($msg, $($args),*)).emit();
            assert!(false);
        }
    };
    ($condition:expr, $span_source:expr, $msg:expr $(,)?) => {
        if !$condition {
            $span_source.span().unwrap().error($msg).emit();
            assert!(false);
        }
    };
    ($condition:expr, $span_source:expr) => {
        assert_spanned_err!($condition, $span_source, concat!("Failed assertion ", stringify!($condition)))
    };
}
