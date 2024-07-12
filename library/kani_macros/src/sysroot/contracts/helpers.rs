// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Functions that operate third party data structures with no logic that is
//! specific to Kani and contracts.

use proc_macro2::{Ident, Span};
use quote::ToTokens;
use std::borrow::Cow;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse_quote, Attribute, Expr, ExprBlock, FnArg, Local, LocalInit, Stmt};

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
/// Create an expression that reconstructs a struct that was matched in a pattern.
///
/// Does not support enums, wildcards, pattern alternatives (`|`), range patterns, or verbatim.
pub fn pat_to_expr(pat: &syn::Pat) -> Expr {
    use syn::Pat;
    let mk_err = |typ| {
        pat.span()
            .unwrap()
            .error(format!("`{typ}` patterns are not supported for functions with contracts"))
            .emit();
        unreachable!()
    };
    match pat {
        Pat::Const(c) => Expr::Const(c.clone()),
        Pat::Ident(id) => Expr::Verbatim(id.ident.to_token_stream()),
        Pat::Lit(lit) => Expr::Lit(lit.clone()),
        Pat::Reference(rf) => Expr::Reference(syn::ExprReference {
            attrs: vec![],
            and_token: rf.and_token,
            mutability: rf.mutability,
            expr: Box::new(pat_to_expr(&rf.pat)),
        }),
        Pat::Tuple(tup) => Expr::Tuple(syn::ExprTuple {
            attrs: vec![],
            paren_token: tup.paren_token,
            elems: tup.elems.iter().map(pat_to_expr).collect(),
        }),
        Pat::Slice(slice) => Expr::Reference(syn::ExprReference {
            attrs: vec![],
            and_token: syn::Token!(&)(Span::call_site()),
            mutability: None,
            expr: Box::new(Expr::Array(syn::ExprArray {
                attrs: vec![],
                bracket_token: slice.bracket_token,
                elems: slice.elems.iter().map(pat_to_expr).collect(),
            })),
        }),
        Pat::Path(pth) => Expr::Path(pth.clone()),
        Pat::Or(_) => mk_err("or"),
        Pat::Rest(_) => mk_err("rest"),
        Pat::Wild(_) => mk_err("wildcard"),
        Pat::Paren(inner) => pat_to_expr(&inner.pat),
        Pat::Range(_) => mk_err("range"),
        Pat::Struct(strct) => {
            if strct.rest.is_some() {
                mk_err("..");
            }
            Expr::Struct(syn::ExprStruct {
                attrs: vec![],
                path: strct.path.clone(),
                brace_token: strct.brace_token,
                dot2_token: None,
                rest: None,
                qself: strct.qself.clone(),
                fields: strct
                    .fields
                    .iter()
                    .map(|field_pat| syn::FieldValue {
                        attrs: vec![],
                        member: field_pat.member.clone(),
                        colon_token: field_pat.colon_token,
                        expr: pat_to_expr(&field_pat.pat),
                    })
                    .collect(),
            })
        }
        Pat::Verbatim(_) => mk_err("verbatim"),
        Pat::Type(pt) => pat_to_expr(pt.pat.as_ref()),
        Pat::TupleStruct(_) => mk_err("tuple struct"),
        _ => mk_err("unknown"),
    }
}

/// Extract the closure arguments which should skip `self`.
///
/// Return the declaration form as well as just a plain list of idents for each.
/// TODO: Handle `mut` arguments.
pub fn closure_args(inputs: &Punctuated<syn::FnArg, Comma>) -> Vec<Expr> {
    closure_params(inputs)
        .map(|arg| {
            if let FnArg::Typed(typed) = arg {
                pat_to_expr(&typed.pat)
            } else {
                unreachable!("Receiver should've been filtered")
            }
        })
        .collect()
}

/// Extract the closure parameters by excluding any receiver.
pub fn closure_params(
    inputs: &Punctuated<syn::FnArg, Comma>,
) -> impl Iterator<Item = &syn::FnArg> + '_ {
    inputs.iter().filter(|arg| matches!(arg, FnArg::Typed(_)))
}

/// Find a closure statement attached with `kanitool::is_contract_generated` attribute.
pub fn find_contract_closure<'a>(stmts: &'a mut [Stmt], name: &'static str) -> &'a mut Stmt {
    let contract = stmts.iter_mut().find(|stmt| {
        if let Stmt::Local(local) = stmt {
            let ident = Ident::new(name, Span::call_site());
            let attr: Attribute = parse_quote!(#[kanitool::is_contract_generated(#ident)]);
            local.attrs.contains(&attr)
        } else {
            false
        }
    });
    contract.expect(&format!("Internal Failure: Expected to find closure `{name}`, but found none"))
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
