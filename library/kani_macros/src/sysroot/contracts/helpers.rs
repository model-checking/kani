// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Functions that operate third party data structures with no logic that is
//! specific to Kani and contracts.

use proc_macro2::{Ident, Span};
use std::borrow::Cow;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprBlock, Local, LocalInit, PatIdent, Stmt, parse_quote};

/// If an explicit return type was provided it is returned, otherwise `()`.
pub fn return_type_to_type(return_type: &syn::ReturnType) -> Cow<'_, syn::Type> {
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
    find_contract_closure(stmts, name).unwrap_or_else(|| {
        panic!("Internal Failure: Expected to find `{name}` closure, but found none")
    })
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
    closure.unwrap_or_else(|| {
        panic!("Internal Failure: Expected to find `{name}` closure, but found none")
    })
}

/// Extract the body of a closure declaration.
pub fn closure_body(closure: &mut Stmt) -> &mut ExprBlock {
    let Stmt::Local(Local { init: Some(LocalInit { expr, .. }), .. }) = closure else {
        unreachable!()
    };
    match expr.as_mut() {
        // The case of closures wrapped in `kani_force_fn_once`
        Expr::Call(call) if call.args.len() == 1 => {
            let arg = call.args.first_mut().unwrap();
            match arg {
                Expr::Closure(closure) => {
                    let Expr::Block(body) = closure.body.as_mut() else { unreachable!() };
                    body
                }
                _ => unreachable!(),
            }
        }

        Expr::Closure(closure) => {
            let Expr::Block(body) = closure.body.as_mut() else { unreachable!() };
            body
        }
        _ => unreachable!(),
    }
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

/// Wrap the evaluation of a contract clause expression so that the
/// kani_core clause-depth counter is incremented around it and its value is
/// returned.
///
/// While a clause is being evaluated, calls to the function whose contract is
/// currently under verification are dispatched to its *original body* instead
/// of its contract *check* (see `FunctionWithContractPass::set_mode` in the
/// Kani compiler).
///
/// The counter is decremented via an RAII guard rather than a direct call so
/// the balancing `exit_contract_clause()` also runs on early exit from the
/// enclosing function (e.g. an `?` or `return` inside `#expr`) and on
/// unwinding panics; otherwise the counter could stay nonzero and mis-
/// dispatch subsequent calls. The guard's `Drop` runs when the block scope
/// ends, i.e. after `#expr` has been fully evaluated.
///
/// `#expr` is bound with a `let` (expression position) rather than left as a
/// block tail expression, so a clause that begins with a block-like construct
/// followed by an operator (e.g. `#[kani::modifies({ .. } as *const _)]` or
/// `#[kani::requires(unsafe { .. } < 100)]`) still parses. Because this moves
/// the whole clause value behind the wrapper block, callers that need the
/// clause value's *span* preserved for diagnostics (i.e. `requires`) instead
/// use [`bracket_clause_stmt`], which keeps the clause expression as a direct
/// argument to `assume`/`assert`.
pub fn bracket_clause_expr(expr: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote::quote!({
        let __kani_clause_guard = kani::internal::enter_contract_clause_guard();
        let __kani_clause_value = #expr;
        __kani_clause_value
    })
}

/// Wrap a contract clause *statement* (e.g. `kani::assume(#cond);` or
/// `kani::assert(#cond, ..);`) in a scope that increments the clause-depth
/// counter for its duration, via the same RAII guard as [`bracket_clause_expr`].
///
/// Unlike [`bracket_clause_expr`], the clause expression is left in place as a
/// direct argument to `assume`/`assert` rather than moved behind a wrapper
/// block. This preserves its span, so that a non-`bool` clause reports the
/// type error on the offending sub-expression rather than on the generated
/// wrapper (see tests/ui/function-contracts/non_bool_contracts.rs and
/// https://github.com/model-checking/kani/issues/3009). The guard is still
/// dropped on all exits from the scope, including unwinding.
pub fn bracket_clause_stmt(stmt: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote::quote!({
        let __kani_clause_guard = kani::internal::enter_contract_clause_guard();
        #stmt
    })
}
