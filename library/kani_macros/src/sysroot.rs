// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::collections::{HashMap, HashSet};

use super::*;

use proc_macro_error::{abort, abort_call_site};
use {
    quote::{format_ident, quote, ToTokens},
    syn::{
        parse::{Parse, ParseStream},
        parse_macro_input,
        spanned::Spanned,
        visit::Visit,
        Expr, ItemFn, PathSegment,
    },
};

use proc_macro2::{Ident, Span};

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

/// Annotate the harness with a #[kanitool::<name>] with optional arguments.
macro_rules! kani_attribute {
    ($name:ident) => {
        pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
            let args = proc_macro2::TokenStream::from(attr);
            let fn_item = parse_macro_input!(item as ItemFn);
            let attribute = format_ident!("{}", stringify!($name));
            quote!(
                #[kanitool::#attribute(#args)]
                #fn_item
            ).into()
        }
    };
    ($name:ident, no_args) => {
        pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
            assert!(attr.is_empty(), "`#[kani::{}]` does not take any arguments currently", stringify!($name));
            let fn_item = parse_macro_input!(item as ItemFn);
            let attribute = format_ident!("{}", stringify!($name));
            quote!(
                #[kanitool::#attribute]
                #fn_item
            ).into()
        }
    };
}

struct ProofOptions {
    schedule: Option<syn::Expr>,
}

impl Parse for ProofOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            Ok(ProofOptions { schedule: None })
        } else {
            let ident = input.parse::<syn::Ident>()?;
            if ident != "schedule" {
                abort_call_site!("`{}` is not a valid option for `#[kani::proof]`.", ident;
                    help = "did you mean `schedule`?";
                    note = "for now, `schedule` is the only option for `#[kani::proof]`.";
                );
            }
            let _ = input.parse::<syn::Token![=]>()?;
            let schedule = Some(input.parse::<syn::Expr>()?);
            Ok(ProofOptions { schedule })
        }
    }
}

pub fn proof(attr: TokenStream, item: TokenStream) -> TokenStream {
    let proof_options = parse_macro_input!(attr as ProofOptions);
    let fn_item = parse_macro_input!(item as ItemFn);
    let attrs = fn_item.attrs;
    let vis = fn_item.vis;
    let sig = fn_item.sig;
    let body = fn_item.block;

    let kani_attributes = quote!(
        #[allow(dead_code)]
        #[kanitool::proof]
    );

    if sig.asyncness.is_none() {
        if proof_options.schedule.is_some() {
            abort_call_site!(
                "`#[kani::proof(schedule = ...)]` can only be used with `async` functions.";
                help = "did you mean to make this function `async`?";
            );
        }
        // Adds `#[kanitool::proof]` and other attributes
        quote!(
            #kani_attributes
            #(#attrs)*
            #vis #sig #body
        )
        .into()
    } else {
        // For async functions, it translates to a synchronous function that calls `kani::block_on`.
        // Specifically, it translates
        // ```ignore
        // #[kani::proof]
        // #[attribute]
        // pub async fn harness() { ... }
        // ```
        // to
        // ```ignore
        // #[kanitool::proof]
        // #[attribute]
        // pub fn harness() {
        //   async fn harness() { ... }
        //   kani::block_on(harness())
        //   // OR
        //   kani::spawnable_block_on(harness(), schedule)
        //   // where `schedule` was provided as an argument to `#[kani::proof]`.
        // }
        // ```
        if !sig.inputs.is_empty() {
            abort!(
                sig.inputs,
                "`#[kani::proof]` cannot be applied to async functions that take arguments for now";
                help = "try removing the arguments";
            );
        }
        let mut modified_sig = sig.clone();
        modified_sig.asyncness = None;
        let fn_name = &sig.ident;
        let schedule = proof_options.schedule;
        let block_on_call = if let Some(schedule) = schedule {
            quote!(kani::block_on_with_spawn(#fn_name(), #schedule))
        } else {
            quote!(kani::block_on(#fn_name()))
        };
        quote!(
            #kani_attributes
            #(#attrs)*
            #vis #modified_sig {
                #sig #body
                #block_on_call
            }
        )
        .into()
    }
}

use syn::visit_mut::VisitMut;

/// Hash this `TokenStream` and return an integer that is at most digits
/// long when hex formatted.
fn short_hash_of_token_stream(stream: &proc_macro::TokenStream) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    hash_of_token_stream(&mut hasher, proc_macro2::TokenStream::from(stream.clone()));
    let long_hash = hasher.finish();
    long_hash % 0x1_000_000 // six hex digits
}

/// Makes consistent names for a generated function which was created for
/// `purpose`, from an attribute that decorates `related_function` with the
/// hash `hash`.
fn identifier_for_generated_function(related_function: &ItemFn, purpose: &str, hash: u64) -> Ident {
    let identifier = format!("{}_{purpose}_{hash:x}", related_function.sig.ident);
    Ident::new(&identifier, proc_macro2::Span::mixed_site())
}

pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    //handle_requires_ensures("requires", false, attr, item)
    requires_ensures_alt(attr, item, true)
}

pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    //handle_requires_ensures("ensures", true, attr, item)
    requires_ensures_alt(attr, item, false)
}

struct SelfDetector(bool);

impl<'ast> Visit<'ast> for SelfDetector {
    fn visit_path(&mut self, i: &'ast syn::Path) {
        self.0 |= i.get_ident().map_or(false, |i| i == "self")
            || i.get_ident().map_or(false, |i| i == "Self")
    }
}

/// Heyristic to determine if this item originated in some kind of `impl`
fn is_probably_impl_fn(item_fn: &ItemFn) -> bool {
    let mut vis = SelfDetector(false);
    vis.visit_signature(&item_fn.sig);
    vis.0
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

/// Applies the contained renaming to everything visited.
struct Renamer<'a>(&'a HashMap<Ident, Ident>);

impl<'a> VisitMut for Renamer<'a> {
    fn visit_expr_path_mut(&mut self, i: &mut syn::ExprPath) {
        if i.path.segments.len() == 1
            && let Some(p) = i.path.segments.first_mut()
            && let Some(new) = self.0.get(&p.ident) {
                p.ident = new.clone();
        }
    }

    /// This restores shadowing. Without this we would rename all ident
    /// occurrences, but not the binding location. This is because our
    /// [`visit_expr_path_mut`] is scope-unaware.
    fn visit_pat_ident_mut(&mut self, i: &mut syn::PatIdent) {
        if let Some(new) = self.0.get(&i.ident) {
            i.ident = new.clone();
        }
    }
}

/// Does the provided path have the same chain of identifiers as `mtch` (match)
/// and no arguments anywhere?
fn matches_path<E>(path: &syn::Path, mtch: &[E]) -> bool
where
    Ident: std::cmp::PartialEq<E>,
{
    path.segments.len() == mtch.len()
        && path.segments.iter().all(|s| s.arguments.is_empty())
        && path.leading_colon.is_none()
        && path.segments.iter().zip(mtch).all(|(actual, expected)| actual.ident == *expected)
}

/// The main meat of handling requires/ensures contracts.
/// 
/// Generates a `check_<fn_name>_<fn_hash>` function that assumes preconditions
/// and asserts postconditions.
/// 
/// Decorates the original function with `#[checked_by =
/// "check_<fn_name>_<fn_hash>"]
/// 
/// Each clause (requires or ensures) creates its own check function that calls
/// the prior check function inside. The innermost check function calls a copy
/// of the originally decorated function. It is a copy, because the compiler
/// later replaces all invocations of the original function with this check and
/// that would also apply to the inner check. We need that to be the untouched
/// function though so we make a copy that will survive the replacement from the
/// compiler.
fn requires_ensures_alt(attr: TokenStream, item: TokenStream, is_requires: bool) -> TokenStream {
    use syn::{FnArg, Pat, PatIdent, PatType};
    let attr_copy = proc_macro2::TokenStream::from(attr.clone());
    let mut attr = parse_macro_input!(attr as Expr);

    let mut output = proc_macro2::TokenStream::new();

    let a_short_hash = short_hash_of_token_stream(&item);

    let item_fn = &mut parse_macro_input!(item as ItemFn);

    let item_name = &item_fn.sig.ident.to_string();

    let mut attrs = std::mem::replace(&mut item_fn.attrs, vec![]);

    let check_fn_name = identifier_for_generated_function(item_fn, "check", a_short_hash);

    let mut check_fn_sig = item_fn.sig.clone();

    check_fn_sig.ident = check_fn_name.clone();

    // This just collects all the arguments and passes them on.
    // TODO: Support patterns.
    let args: Vec<_> = item_fn
        .sig
        .inputs
        .iter()
        .map(|arg| {
            Expr::Path(syn::ExprPath {
                attrs: vec![],
                qself: None,
                path: syn::Path {
                    leading_colon: None,
                    segments: [PathSegment {
                        ident: match arg {
                            FnArg::Receiver(_) => Ident::new("self", Span::call_site()),
                            FnArg::Typed(PatType { pat, .. }) => match &**pat {
                                Pat::Ident(PatIdent { ident, .. }) => ident.clone(),
                                _ => {
                                    pat.span().unwrap().error("Unexpected pattern").emit();
                                    unreachable!()
                                }
                            },
                        },
                        arguments: syn::PathArguments::None,
                    }]
                    .into_iter()
                    .collect(),
                },
            })
        })
        .collect();
    let is_impl_fn = is_probably_impl_fn(item_fn);

    let mut prior_check = None;

    // We remove any prior `checked_with` or `replaced_with` and use the
    // identifiers stored there as the inner functions we will call to in the
    // checking and replacing track respectively.
    //
    // This also maintains the invariant that there is always at most one
    // annotation each of `kanitool::{checked_with, replaced_with}` on the
    // function, e.g. there is a canonical check/replace.
    attrs.retain(|attr| {
        if let syn::Meta::NameValue(nv) = &attr.meta {
            if matches_path(&nv.path, &["kanitool", "checked_with"]) {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(strlit), .. }) = &nv.value {
                    assert!(prior_check.replace(strlit.value()).is_none());
                    return false;
                }
            }
        }
        true
    });

    let prior_check_fn_name = prior_check.map_or_else(
        || {
            // If there was no prior check we make a copy and use its name.
            let copy_fn_name = identifier_for_generated_function(&item_fn, "copy", a_short_hash);
            let mut copy_fn = item_fn.clone();
            copy_fn.sig.ident = copy_fn_name.clone();
            output.extend(copy_fn.into_token_stream());
            copy_fn_name
        },
        |prior| Ident::new(&prior, Span::call_site()),
    );

    let call_to_prior = if is_impl_fn {
        // If we're in an `impl`, we need to call it with `Self::`
        quote!(Self::#prior_check_fn_name(#(#args),*))
    } else {
        quote!(#prior_check_fn_name(#(#args),*))
    };

    let check_body = if is_requires {
        quote!(
            kani::assume(#attr);
            #call_to_prior
        )
    } else {
        let mut arg_ident_collector = ArgumentIdentCollector::new();
        arg_ident_collector.visit_signature(&item_fn.sig);

        let mk_new_ident_for =
            |id: &Ident| Ident::new(&format!("{}_renamed", id), Span::mixed_site());
        let arg_idents = arg_ident_collector
            .0
            .into_iter()
            .map(|i| {
                let new = mk_new_ident_for(&i);
                (i, new)
            })
            .collect::<HashMap<_, _>>();

        let mut ident_rewriter = Renamer(&arg_idents);
        ident_rewriter.visit_expr_mut(&mut attr);

        let arg_copy_names = arg_idents.values();
        let arg_idents = arg_idents.keys();

        quote!(
            #(let #arg_copy_names = kani::untracked_deref(&#arg_idents);)*
            let result = #call_to_prior;
            kani::assert(#attr, stringify!(#attr_copy));
            result
        )
    };

    // Constructing string literals explicitly here, because if we call
    // `stringify!` in the generated code that is passed on as that expression to
    // the next expansion of a contract, not as the literal.
    let check_fn_name_str = syn::LitStr::new(&check_fn_name.to_string(), Span::call_site());

    // The order of `attrs` and `kanitool::checked_with` is
    // important here, because macros are expanded outside in. This way other
    // contract annotations in `attrs` sees the `checked_with`
    // attribute and can use them.
    //
    // This way we generate a clean chain of checking and replacing calls.
    output.extend(quote!(
        #(#attrs)*
        #[kanitool::checked_with = #check_fn_name_str]
        #item_fn

        #[allow(dead_code)]
        #[allow(unused_variables)]
        #check_fn_sig {
            #check_body
        }
    ));
    output.into()
}

macro_rules! passthrough {
    ($name:ident, $allow_dead_code:ident) => {
        pub fn $name(attr: TokenStream, item: TokenStream) -> TokenStream {
            let args = proc_macro2::TokenStream::from(attr);
            let fn_item = proc_macro2::TokenStream::from(item);
            let name = Ident::new(stringify!($name), proc_macro2::Span::call_site());
            let extra_attrs = if $allow_dead_code {
                quote!(#[allow(dead_code)])
            } else {
                quote!()
            };
            quote!(
                #extra_attrs
                #[kanitool::#name = stringify!(#args)]
                #fn_item
            )
            .into()
        }
    }
}

passthrough!(proof_for_contract, true);

kani_attribute!(should_panic, no_args);
kani_attribute!(solver);
kani_attribute!(stub);
kani_attribute!(unstable);
kani_attribute!(unwind);
