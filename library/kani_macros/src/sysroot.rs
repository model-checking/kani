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
        Expr, ItemFn,
    },
};

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};

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

/// Applies the contained renaming to every ident pattern and ident expr
/// visited.
///
/// In each case if the path has only one segments that corresponds to a key, it
/// is replaced by the ident in the value.
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

/// Classifies the state a function is in in the contract handling pipeline.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ContractFunctionState {
    /// This is the original code, re-emitted from a contract attribute
    Original,
    /// This is the first time a contract attribute is evaluated on this
    /// function
    Untouched,
    /// This is a check function that was generated from a previous evaluation
    /// of a contract attribute
    Check,
}

impl ContractFunctionState {
    /// Find out if this attribute could be describing a "contract handling"
    /// state and if so return it.
    fn from_attribute(attribute: &syn::Attribute) -> Option<Self> {
        if let syn::Meta::List(lst) = &attribute.meta {
            if matches_path(&lst.path, &["kanitool", "is_contract_generated"]) {
                match syn::parse2::<Ident>(lst.tokens.clone()) {
                    Err(e) => {
                        lst.span().unwrap().error(format!("{e}")).emit();
                    }
                    Ok(ident) => {
                        if ident.to_string() == "check" {
                            return Some(Self::Check);
                        } else {
                            lst.span().unwrap().error("Expected `check` ident").emit();
                        }
                    }
                }
            }
        }
        if let syn::Meta::NameValue(nv) = &attribute.meta {
            if matches_path(&nv.path, &["kanitool", "checked_with"]) {
                return Some(ContractFunctionState::Original);
            }
        }
        None
    }
}

/// A visitor which injects a copy of the token stream it holds before every
/// `return` expression.
///
/// This is intended to be used with postconditions and for that purpose it also
/// performs a rewrite where the return value is first bound to `result` so the
/// postconditions can access it.
///
/// # Example
///
/// The expression `return x;` turns into
///
/// ```rs
/// { // Always opens a new block
///     let result = x;
///     <injected tokenstream>
///     return result;
/// }
/// ```
struct PostconditionInjector(TokenStream2);

impl VisitMut for PostconditionInjector {
    fn visit_expr_closure_mut(&mut self, _: &mut syn::ExprClosure) {
        // Empty because inside the closure we don't want to inject
    }

    fn visit_expr_mut(&mut self, i: &mut Expr) {
        if let syn::Expr::Return(r) = i {
            let tokens = self.0.clone();
            let mut output = TokenStream2::new();
            if let Some(expr) = &mut r.expr {
                output.extend(quote!(let result = #expr;));
                *expr = Box::new(Expr::Verbatim(quote!(result)));
            }
            *i = syn::Expr::Verbatim(quote!({
                #output
                #tokens
                #i
            }))
        } else {
            syn::visit_mut::visit_expr_mut(self, i)
        }
    }
}

/// A supporting function for creating shallow, unsafe copies of the arguments
/// for the postconditions.
///
/// This function
/// - collects all [`Ident`]s found in the argument patterns
/// - creates new names for them
/// - Replaces all occurrences of those idents in `attrs` with the new names and
/// - Returns the mapping of old names to new names so the macro can emit the
///   code that makes the copies.
fn rename_argument_occurences(sig: &syn::Signature, attr: &mut Expr) -> HashMap<Ident, Ident> {
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

/// The main meat of handling requires/ensures contracts.
///
/// Generates a `check_<fn_name>_<fn_hash>` function that assumes preconditions
/// and asserts postconditions.
///
/// Decorates the original function with `#[checked_by =
/// "check_<fn_name>_<fn_hash>"]
///
/// The check function is a copy of the original function with pre and
/// postconditions added before and after respectively. Each clause (requires or
/// ensures) adds new layers of pre or postconditions to the check function.
///
/// Postconditions are also injected before every `return` expression, see also
/// [`PostconditionInjector`].
///
/// All arguments of the function are unsafely shallow-copied with the
/// `kani::untracked_deref` function. We must ensure that those copies are not
/// dropped so after the postconditions we call `mem::forget` on each copy.
///
/// # Complete example
///
/// ```rs
///
/// ```
///
/// Turns into
///
/// ```rs
/// #[kanitool::checked_with = "div_check_965916"]
/// fn div(dividend: u32, divisor: u32) -> u32 { dividend / divisor }
///
/// #[allow(dead_code)]
/// #[allow(unused_variables)]
/// #[kanitool::is_contract_generated(check)]
/// fn div_check_965916(dividend: u32, divisor: u32) -> u32 {
///     let dividend_renamed = kani::untracked_deref(&dividend);
///     let divisor_renamed = kani::untracked_deref(&divisor);
///     let result = { kani::assume(divisor != 0); { dividend / divisor } };
///     kani::assert(result <= dividend_renamed, "result <= dividend");
///     std::mem::forget(dividend_renamed);
///     std::mem::forget(divisor_renamed);
///     result
/// }
/// ```
fn requires_ensures_alt(attr: TokenStream, item: TokenStream, is_requires: bool) -> TokenStream {
    let attr_copy = proc_macro2::TokenStream::from(attr.clone());
    let mut attr = parse_macro_input!(attr as Expr);

    let mut output = proc_macro2::TokenStream::new();

    let a_short_hash = short_hash_of_token_stream(&item);
    let item_fn = &mut parse_macro_input!(item as ItemFn);

    // If we didn't find any other contract handling related attributes we
    // assume this function has not been touched by a contract before.
    let function_state = item_fn
        .attrs
        .iter()
        .find_map(ContractFunctionState::from_attribute)
        .unwrap_or(ContractFunctionState::Untouched);

    if matches!(function_state, ContractFunctionState::Original) {
        // If we're the original function that means we're *not* the first time
        // that a contract attribute is handled on this function. This means
        // there must exist a generated check function somewhere onto which the
        // attributes have been copied and where they will be expanded into more
        // checks. So we just return outselves unchanged.
        return item_fn.into_token_stream().into();
    }

    let attrs = std::mem::replace(&mut item_fn.attrs, vec![]);

    if matches!(function_state, ContractFunctionState::Untouched) {
        // We are the first time a contract is handled on this function, so
        // we're responsible for
        //
        // 1. Generating a name for the check function
        // 2. Emitting the original, unchanged item and register the check
        //    function on it via attribute
        // 3. Renaming our item to the new name
        // 4. And (minor point) adding #[allow(dead_code)] and
        //    #[allow(unused_variables)] to the check function attributes

        // The order of `attrs` and `kanitool::{checked_with,
        // is_contract_generated}` is important here, because macros are
        // expanded outside in. This way other contract annotations in
        // `attrs` sees those attribuites which they need to determine
        // `function_state` attribute and can use them.
        //
        // The same applies later when we emit the check function.
        let check_fn_name = identifier_for_generated_function(item_fn, "check", a_short_hash);

        // Constructing string literals explicitly here, because if we call
        // `stringify!` in the generated code that is passed on as that
        // expression to the next expansion of a contract, not as the
        // literal.
        let check_fn_name_str = syn::LitStr::new(&check_fn_name.to_string(), Span::call_site());
        output.extend(quote!(
            #(#attrs)*
            #[kanitool::checked_with = #check_fn_name_str]
            #item_fn

            #[allow(dead_code)]
            #[allow(unused_variables)]
        ));
        item_fn.sig.ident = check_fn_name;
    }

    let call_to_prior = &mut item_fn.block;

    let check_body = if is_requires {
        quote!(
            kani::assume(#attr);
            #call_to_prior
        )
    } else {
        let arg_idents = rename_argument_occurences(&item_fn.sig, &mut attr);

        let arg_copy_names = arg_idents.values();
        let also_arg_copy_names = arg_copy_names.clone();
        let arg_idents = arg_idents.keys();

        let exec_postconditions = quote!(
            kani::assert(#attr, stringify!(#attr_copy));
            #(std::mem::forget(#also_arg_copy_names);)*
        );

        let mut inject_conditions = PostconditionInjector(exec_postconditions.clone());

        inject_conditions.visit_block_mut(&mut *call_to_prior);

        quote!(
            #(let #arg_copy_names = kani::untracked_deref(&#arg_idents);)*
            let result = #call_to_prior;
            #exec_postconditions
            result
        )
    };

    let sig = &item_fn.sig;

    output.extend(quote!(
        #(#attrs)*
    ));

    if matches!(function_state, ContractFunctionState::Untouched) {
        output.extend(quote!(#[kanitool::is_contract_generated(check)]));
    }

    // Finally emit the check function.
    output.extend(quote!(
        #sig {
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
