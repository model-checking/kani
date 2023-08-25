// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::collections::{HashMap, HashSet};

use proc_macro::TokenStream;

use {
    quote::{quote, ToTokens},
    syn::{parse_macro_input, spanned::Spanned, visit::Visit, Expr, ItemFn},
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
    requires_ensures_alt(attr, item, true)
}

pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
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
    /// [`visit_expr_path_mut`] is scope-unaware.
    fn visit_pat_ident_mut(&mut self, i: &mut syn::PatIdent) {
        if let Some(new) = self.0.get(&i.ident) {
            i.ident = new.clone();
        }
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
    /// We leave this emtpy to stop the recursion here. We don't want to look
    /// inside the closure, because the return statements contained within are
    /// for a different function, duh.
    fn visit_expr_closure_mut(&mut self, _: &mut syn::ExprClosure) {}

    fn visit_expr_mut(&mut self, i: &mut Expr) {
        if let syn::Expr::Return(r) = i {
            let tokens = self.0.clone();
            let mut output = TokenStream2::new();
            if let Some(expr) = &mut r.expr {
                // In theory the return expression can contain itself a `return`
                // so we need to recurse here.
                self.visit_expr_mut(expr);
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
/// - Collects all [`Ident`]s found in the argument patterns
/// - Creates new names for them
/// - Replaces all occurrences of those idents in `attrs` with the new names and
/// - Returns the mapping of old names to new names
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
/// and asserts postconditions. The check function is also marked as generated
/// with the `#[kanitool::is_contract_generated(check)]` attribute.
///
/// Decorates the original function with `#[kanitool::checked_by =
/// "check_<fn_name>_<fn_hash>"]
///
/// The check function is a copy of the original function with preconditions
/// added before the body and postconditions after as well as injected before
/// every `return` (see [`PostconditionInjector`]). Attributes on the original
/// function are also copied to the check function. Each clause (requires or
/// ensures) after the first will be ignored on the original function (detected
/// by finding the `kanitool::checked_with` attribute). On the check function
/// (detected by finding the `kanitool::is_contract_generated` attribute) it
/// expands into a new layer of pre- or postconditions. This state machine is
/// also explained in more detail in comments in the body of this macro.
///
/// In the check function all named arguments of the function are unsafely
/// shallow-copied with the `kani::untracked_deref` function to circumvent the
/// borrow checker for postconditions. We must ensure that those copies are not
/// dropped so after the postconditions we call `mem::forget` on each copy.
///
/// # Complete example
///
/// ```rs
/// #[kani::requires(divisor != 0)]
/// #[kani::ensures(result <= dividend)]
/// fn div(dividend: u32, divisor: u32) -> u32 {
///     dividend / divisor
/// }
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

        let check_fn_name = identifier_for_generated_function(item_fn, "check", a_short_hash);

        // Constructing string literals explicitly here, because if we call
        // `stringify!` in the generated code that is passed on as that
        // expression to the next expansion of a contract, not as the
        // literal.
        let check_fn_name_str = syn::LitStr::new(&check_fn_name.to_string(), Span::call_site());

        // The order of `attrs` and `kanitool::{checked_with,
        // is_contract_generated}` is important here, because macros are
        // expanded outside in. This way other contract annotations in `attrs`
        // sees those attribuites and can use them to determine
        // `function_state`.
        //
        // We're emitting the original here but the same applies later when we
        // emit the check function.
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

        // The code that enforces the postconditions and cleans up the shallow
        // argument copies (with `mem::forget`).
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

    // Prepare emitting the check function by emitting the rest of the
    // attributes.
    output.extend(quote!(
        #(#attrs)*
    ));

    if matches!(function_state, ContractFunctionState::Untouched) {
        // If it's the first time we also emit this marker. Again, order is
        // important so this happens as the last emitted attribute.
        output.extend(quote!(#[kanitool::is_contract_generated(check)]));
    }

    // Finally emit the check function itself.
    output.extend(quote!(
        #sig {
            #check_body
        }
    ));
    output.into()
}

/// This is very similar to the kani_attribute macro, but it instead creates
/// key-value style attributes which I find a little easier to parse.
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
