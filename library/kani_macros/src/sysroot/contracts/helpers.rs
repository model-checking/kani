// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Functions that operate third party data structures with no logic that is
//! specific to Kani and contracts.

use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use std::borrow::Cow;
use syn::{spanned::Spanned, visit::Visit, Expr, FnArg, ItemFn};

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

/// For each argument create an expression that passes this argument along unmodified.
///
/// Reconstructs structs that may have been deconstructed with patterns.
pub fn exprs_for_args<T>(
    args: &syn::punctuated::Punctuated<FnArg, T>,
) -> impl Iterator<Item = Expr> + Clone + '_ {
    args.iter().map(|arg| match arg {
        FnArg::Receiver(_) => Expr::Verbatim(quote!(self)),
        FnArg::Typed(typed) => pat_to_expr(&typed.pat),
    })
}

/// The visitor used by [`is_probably_impl_fn`]. See function documentation for
/// more information.
struct SelfDetector(bool);

impl<'ast> Visit<'ast> for SelfDetector {
    fn visit_ident(&mut self, i: &'ast syn::Ident) {
        self.0 |= i == &Ident::from(syn::Token![Self](Span::mixed_site()))
    }

    fn visit_receiver(&mut self, _node: &'ast syn::Receiver) {
        self.0 = true;
    }
}

/// Try to determine if this function is part of an `impl`.
///
/// Detects *methods* by the presence of a receiver argument. Heuristically
/// detects *associated functions* by the use of `Self` anywhere.
///
/// Why do we need this? It's because if we want to call this `fn`, or any other
/// `fn` we generate into the same context we need to use `foo()` or
/// `Self::foo()` respectively depending on whether this is a plain or
/// associated function or Rust will complain. For the contract machinery we
/// need to generate and then call various functions we generate as well as the
/// original contracted function and so we need to determine how to call them
/// correctly.
///
/// We can only solve this heuristically. The fundamental problem with Rust
/// macros is that they only see the syntax that's given to them and no other
/// context. It is however that context (of an `impl` block) that definitively
/// determines whether the `fn` is a plain function or an associated function.
///
/// The heuristic itself is flawed, but it's the best we can do. For instance
/// this is perfectly legal
///
/// ```
/// struct S;
/// impl S {
///     #[i_want_to_call_you]
///     fn helper(u: usize) -> bool {
///       u < 8
///     }
///   }
/// ```
///
/// This function would have to be called `S::helper()` but to the
/// `#[i_want_to_call_you]` attribute this function looks just like a bare
/// function because it never mentions `self` or `Self`. While this is a rare
/// case, the following is much more likely and suffers from the same problem,
/// because we can't know that `Vec == Self`.
///
/// ```
/// impl<T> Vec<T> {
///   fn new() -> Vec<T> {
///     Vec { cap: 0, buf: NonNull::dangling() }
///   }
/// }
/// ```
///
/// **Side note:** You may be tempted to suggest that we could try and parse
/// `syn::ImplItemFn` and distinguish that from `syn::ItemFn` to distinguish
/// associated function from plain functions. However parsing in an attribute
/// placed on *any* `fn` will always succeed for *both* `syn::ImplItemFn` and
/// `syn::ItemFn`, thus not letting us distinguish between the two.
pub fn is_probably_impl_fn(fun: &ItemFn) -> bool {
    let mut self_detector = SelfDetector(false);
    self_detector.visit_item_fn(fun);
    self_detector.0
}

/// Convert every use of a pattern in this signature to a simple, fresh, binding-only
/// argument ([`syn::PatIdent`]) and return the [`Ident`] that was generated.
pub fn pats_to_idents<P>(
    sig: &mut syn::punctuated::Punctuated<syn::FnArg, P>,
) -> impl Iterator<Item = Ident> + '_ {
    sig.iter_mut().enumerate().map(|(i, arg)| match arg {
        syn::FnArg::Receiver(_) => Ident::from(syn::Token![self](Span::call_site())),
        syn::FnArg::Typed(syn::PatType { pat, .. }) => {
            let ident = Ident::new(&format!("arg{i}"), Span::mixed_site());
            *pat.as_mut() = syn::Pat::Ident(syn::PatIdent {
                attrs: vec![],
                by_ref: None,
                mutability: None,
                ident: ident.clone(),
                subpat: None,
            });
            ident
        }
    })
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

/// Create a unique hash for a token stream (basically a [`std::hash::Hash`]
/// impl for `proc_macro2::TokenStream`).
fn hash_of_token_stream<H: std::hash::Hasher>(hasher: &mut H, stream: proc_macro2::TokenStream) {
    use proc_macro2::TokenTree;
    use std::hash::Hash;
    for token in stream {
        match token {
            TokenTree::Ident(i) => i.hash(hasher),
            TokenTree::Punct(p) => p.as_char().hash(hasher),
            TokenTree::Group(g) => {
                std::mem::discriminant(&g.delimiter()).hash(hasher);
                hash_of_token_stream(hasher, g.stream());
            }
            TokenTree::Literal(lit) => lit.to_string().hash(hasher),
        }
    }
}

/// Hash this `TokenStream` and return an integer that is at most digits
/// long when hex formatted.
pub fn short_hash_of_token_stream(stream: &proc_macro::TokenStream) -> u64 {
    const SIX_HEX_DIGITS_MASK: u64 = 0x1_000_000;
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    hash_of_token_stream(&mut hasher, proc_macro2::TokenStream::from(stream.clone()));
    let long_hash = hasher.finish();
    long_hash % SIX_HEX_DIGITS_MASK
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