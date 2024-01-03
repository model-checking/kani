// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implementation of the function contracts code generation.
//!
//! The most exciting part is the handling of `requires` and `ensures`, the main
//! entry point to which is [`requires_ensures_main`]. Most of the code
//! generation for that is implemented on [`ContractConditionsHandler`] with
//! [`ContractFunctionState`] steering the code generation. The function state
//! implements a state machine in order to be able to handle multiple attributes
//! on the same function correctly.
//!
//! ## How the handling for `requires` and `ensures` works.
//!
//! Our aim is to generate a "check" function that can be used to verify the
//! validity of the contract and a "replace" function that can be used as a
//! stub, generated from the contract that can be used instead of the original
//! function.
//!
//! Let me first introduce the constraints which we are operating under to
//! explain why we need the somewhat involved state machine to achieve this.
//!
//! Proc-macros are expanded one-at-a-time, outside-in and they can also be
//! renamed. Meaning the user can do `use kani::requires as precondition` and
//! then use `precondition` everywhere.  We want to support this functionality
//! instead of throwing a hard error but this means we cannot detect if a given
//! function has further contract attributes placed on it during any given
//! expansion. As a result every expansion needs to leave the code in a valid
//! state that could be used for all contract functionality but it must alow
//! further contract attributes to compose with what was already generated. In
//! addition we also want to make sure to support non-contract attributes on
//! functions with contracts.
//!
//! To this end we use a state machine. The initial state is an "untouched"
//! function with possibly multiple contract attributes, none of which have been
//! expanded. When we expand the first (outermost) `requires` or `ensures`
//! attribute on such a function we re-emit the function unchanged but we also
//! generate fresh "check" and "replace" functions that enforce the condition
//! carried by the attribute currently being expanded. We copy all additional
//! attributes from the original function to both the "check" and the "replace".
//! This allows us to deal both with renaming and also support non-contract
//! attributes.
//!
//! In addition to copying attributes we also add new marker attributes to
//! advance the state machine. The "check" function gets a
//! `kanitool::is_contract_generated(check)` attributes and analogous for
//! replace. The re-emitted original meanwhile is decorated with
//! `kanitool::checked_with(name_of_generated_check_function)` and an analogous
//! `kanittool::replaced_with` attribute also. The next contract attribute that
//! is expanded will detect the presence of these markers in the attributes of
//! the item and be able to determine their position in the state machine this
//! way. If the state is either a "check" or "replace" then the body of the
//! function is augmented with the additional conditions carried by the macro.
//! If the state is the "original" function, no changes are performed.
//!
//! We place marker attributes at the bottom of the attribute stack (innermost),
//! otherwise they would not be visible to the future macro expansions.
//!
//! Below you can see a graphical rendering where boxes are states and each
//! arrow represents the expansion of a `requires` or `ensures` macro.
//!
//! ```plain
//!                           │ Start
//!                           ▼
//!                     ┌───────────┐
//!                     │ Untouched │
//!                     │ Function  │
//!                     └─────┬─────┘
//!                           │
//!            Emit           │  Generate      + Copy Attributes
//!         ┌─────────────────┴─────┬──────────┬─────────────────┐
//!         │                       │          │                 │
//!         │                       │          │                 │
//!         ▼                       ▼          ▼                 ▼
//!  ┌──────────┐           ┌───────────┐  ┌───────┐        ┌─────────┐
//!  │ Original │◄─┐        │ Recursion │  │ Check │◄─┐     │ Replace │◄─┐
//!  └──┬───────┘  │        │ Wrapper   │  └───┬───┘  │     └────┬────┘  │
//!     │          │ Ignore └───────────┘      │      │ Augment  │       │ Augment
//!     └──────────┘                           └──────┘          └───────┘
//!
//! │               │       │                                             │
//! └───────────────┘       └─────────────────────────────────────────────┘
//!
//!     Presence of                            Presence of
//!    "checked_with"                    "is_contract_generated"
//!
//!                        State is detected via
//! ```
//!
//! All named arguments of the annotated function are unsafely shallow-copied
//! with the `kani::untracked_deref` function to circumvent the borrow checker
//! for postconditions. The case where this is relevant is if you want to return
//! a mutable borrow from the function which means any immutable borrow in the
//! postcondition would be illegal. We must ensure that those copies are not
//! dropped (causing a double-free) so after the postconditions we call
//! `mem::forget` on each copy.
//!
//! ## Check function
//!
//! Generates a `<fn_name>_check_<fn_hash>` function that assumes preconditions
//! and asserts postconditions. The check function is also marked as generated
//! with the `#[kanitool::is_contract_generated(check)]` attribute.
//!
//! Decorates the original function with `#[kanitool::checked_by =
//! "<fn_name>_check_<fn_hash>"]`.
//!
//! The check function is a copy of the original function with preconditions
//! added before the body and postconditions after as well as injected before
//! every `return` (see [`PostconditionInjector`]). Attributes on the original
//! function are also copied to the check function.
//!
//! ## Replace Function
//!
//! As the mirror to that also generates a `<fn_name>_replace_<fn_hash>`
//! function that asserts preconditions and assumes postconditions. The replace
//! function is also marked as generated with the
//! `#[kanitool::is_contract_generated(replace)]` attribute.
//!
//! Decorates the original function with `#[kanitool::replaced_by =
//! "<fn_name>_replace_<fn_hash>"]`.
//!
//! The replace function has the same signature as the original function but its
//! body is replaced by `kani::any()`, which generates a non-deterministic
//! value.
//!
//! ## Inductive Verification
//!
//! To efficiently check recursive functions we verify them inductively. To
//! be able to do this we need both the check and replace functions we have seen
//! before.
//!
//! Inductive verification is comprised of a hypothesis and an induction step.
//! The hypothesis in this case is the replace function. It represents the
//! assumption that the contracts holds if the preconditions are satisfied. The
//! induction step is the check function, which ensures that the contract holds,
//! assuming the preconditions hold.
//!
//! Since the induction revolves around the recursive call we can simply set it
//! up upon entry into the body of the function under verification. We use a
//! global variable that tracks whether we are re-entering the function
//! recursively and starts off as `false`. On entry to the function we flip the
//! variable to `true` and dispatch to the check (induction step). If the check
//! recursively calls our function our re-entry tracker now reads `true` and we
//! dispatch to the replacement (application of induction hypothesis). Because
//! the replacement function only checks the conditions and does not perform
//! other computation we will only ever go "one recursion level deep", making
//! inductive verification very efficient. Once the check function returns we
//! flip the tracker variable back to `false` in case the function is called
//! more than once in its harness.
//!
//! To facilitate all this we generate a `<fn_name>_recursion_wrapper_<fn_hash>`
//! function with the following shape:
//!
//! ```ignored
//! fn recursion_wrapper_...(fn args ...) {
//!     static mut REENTRY: bool = false;
//!
//!     if unsafe { REENTRY } {
//!         call_replace(fn args...)
//!     } else {
//!         unsafe { reentry = true };
//!         let result = call_check(fn args...);
//!         unsafe { reentry = false };
//!         result
//!     }
//! }
//! ```
//!
//! We register this function as `#[kanitool::checked_with =
//! "recursion_wrapper_..."]` instead of the check function.
//!
//! # Complete example
//!
//! ```
//! #[kani::requires(divisor != 0)]
//! #[kani::ensures(result <= dividend)]
//! fn div(dividend: u32, divisor: u32) -> u32 {
//!     dividend / divisor
//! }
//! ```
//!
//! Turns into
//!
//! ```
//! #[kanitool::checked_with = "div_recursion_wrapper_965916"]
//! #[kanitool::replaced_with = "div_replace_965916"]
//! fn div(dividend: u32, divisor: u32) -> u32 { dividend / divisor }
//!
//! #[allow(dead_code)]
//! #[allow(unused_variables)]
//! #[kanitool::is_contract_generated(check)]
//! fn div_check_965916(dividend: u32, divisor: u32) -> u32 {
//!     let dividend_renamed = kani::untracked_deref(&dividend);
//!     let divisor_renamed = kani::untracked_deref(&divisor);
//!     let result = { kani::assume(divisor != 0); { dividend / divisor } };
//!     kani::assert(result <= dividend_renamed, "result <= dividend");
//!     std::mem::forget(dividend_renamed);
//!     std::mem::forget(divisor_renamed);
//!     result
//! }
//!
//! #[allow(dead_code)]
//! #[allow(unused_variables)]
//! #[kanitool::is_contract_generated(replace)]
//! fn div_replace_965916(dividend: u32, divisor: u32) -> u32 {
//!     kani::assert(divisor != 0, "divisor != 0");
//!     let dividend_renamed = kani::untracked_deref(&dividend);
//!     let divisor_renamed = kani::untracked_deref(&divisor);
//!     let result = kani::any();
//!     kani::assume(result <= dividend_renamed, "result <= dividend");
//!     std::mem::forget(dividend_renamed);
//!     std::mem::forget(divisor_renamed);
//!     result
//! }
//!
//! #[allow(dead_code)]
//! #[allow(unused_variables)]
//! #[kanitool::is_contract_generated(recursion_wrapper)]
//! fn div_recursion_wrapper_965916(dividend: u32, divisor: u32) -> u32 {
//!     static mut REENTRY: bool = false;
//!
//!     if unsafe { REENTRY } {
//!         div_replace_965916(dividend, divisor)
//!     } else {
//!         unsafe { reentry = true };
//!         let result = div_check_965916(dividend, divisor);
//!         unsafe { reentry = false };
//!         result
//!     }
//! }
//! ```

use proc_macro::{Diagnostic, TokenStream};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};
use syn::{
    parse_macro_input, spanned::Spanned, visit::Visit, visit_mut::VisitMut, Attribute, Expr,
    ItemFn, PredicateType, ReturnType, Signature, TraitBound, TypeParamBound, WhereClause,
};

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
    requires_ensures_main(attr, item, true)
}

pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_main(attr, item, false)
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
    /// This is the original code, re-emitted from a contract attribute.
    Original,
    /// This is the first time a contract attribute is evaluated on this
    /// function.
    Untouched,
    /// This is a check function that was generated from a previous evaluation
    /// of a contract attribute.
    Check,
    /// This is a replace function that was generated from a previous evaluation
    /// of a contract attribute.
    Replace,
}

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
    fn from_attributes(attributes: &[syn::Attribute]) -> Self {
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

    /// Do we need to emit the `is_contract_generated` tag attribute on the
    /// generated function(s)?
    fn emit_tag_attr(self) -> bool {
        matches!(self, ContractFunctionState::Untouched)
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
    /// We leave this empty to stop the recursion here. We don't want to look
    /// inside the closure, because the return statements contained within are
    /// for a different function.
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

/// The information needed to generate the bodies of check and replacement
/// functions that integrate the conditions from this contract attribute.
struct ContractConditionsHandler<'a> {
    function_state: ContractFunctionState,
    /// Information specific to the type of contract attribute we're expanding.
    condition_type: ContractConditionsType,
    /// The contents of the attribute.
    attr: Expr,
    /// Body of the function this attribute was found on.
    annotated_fn: &'a ItemFn,
    /// An unparsed, unmodified copy of `attr`, used in the error messages.
    attr_copy: TokenStream2,
    /// The stream to which we should write the generated code.
    output: &'a mut TokenStream2,
}

/// Information needed for generating check and replace handlers for different
/// contract attributes.
enum ContractConditionsType {
    Requires,
    Ensures {
        /// Translation map from original argument names to names of the copies
        /// we will be emitting.
        argument_names: HashMap<Ident, Ident>,
    },
}

impl ContractConditionsType {
    /// Constructs a [`Self::Ensures`] from the signature of the decorated
    /// function and the contents of the decorating attribute.
    ///
    /// Renames the [`Ident`]s used in `attr` and stores the translation map in
    /// `argument_names`.
    fn new_ensures(sig: &Signature, attr: &mut Expr) -> Self {
        let argument_names = rename_argument_occurrences(sig, attr);
        ContractConditionsType::Ensures { argument_names }
    }
}

impl<'a> ContractConditionsHandler<'a> {
    /// Initialize the handler. Constructs the required
    /// [`ContractConditionsType`] depending on `is_requires`.
    fn new(
        function_state: ContractFunctionState,
        is_requires: bool,
        mut attr: Expr,
        annotated_fn: &'a ItemFn,
        attr_copy: TokenStream2,
        output: &'a mut TokenStream2,
    ) -> Self {
        let condition_type = if is_requires {
            ContractConditionsType::Requires
        } else {
            ContractConditionsType::new_ensures(&annotated_fn.sig, &mut attr)
        };

        Self { function_state, condition_type, attr, annotated_fn, attr_copy, output }
    }

    /// Create the body of a check function.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    fn make_check_body(&self) -> TokenStream2 {
        let Self { attr, attr_copy, .. } = self;
        let ItemFn { sig, block, .. } = self.annotated_fn;
        let return_type = return_type_to_type(&sig.output);

        match &self.condition_type {
            ContractConditionsType::Requires => quote!(
                kani::assume(#attr);
                #block
            ),
            ContractConditionsType::Ensures { argument_names } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);

                // The code that enforces the postconditions and cleans up the shallow
                // argument copies (with `mem::forget`).
                let exec_postconditions = quote!(
                    kani::assert(#attr, stringify!(#attr_copy));
                    #copy_clean
                );

                // We make a copy here because we'll modify it. Technically not
                // necessary but could lead to weird results if
                // `make_replace_body` were called after this if we modified in
                // place.
                let mut call = block.clone();
                let mut inject_conditions = PostconditionInjector(exec_postconditions.clone());
                inject_conditions.visit_block_mut(&mut call);

                quote!(
                    #arg_copies
                    let result : #return_type = #call;
                    #exec_postconditions
                    result
                )
            }
        }
    }

    /// Create the body of a stub for this contract.
    ///
    /// Wraps the conditions from this attribute around a prior call. If
    /// `use_nondet_result` is `true` we will use `kani::any()` to create a
    /// result, otherwise whatever the `body` of our annotated function was.
    ///
    /// `use_nondet_result` will only be true if this is the first time we are
    /// generating a replace function.
    fn make_replace_body(&self, use_nondet_result: bool) -> TokenStream2 {
        let Self { attr, attr_copy, .. } = self;
        let ItemFn { sig, block, .. } = self.annotated_fn;
        let call_to_prior =
            if use_nondet_result { quote!(kani::any()) } else { block.to_token_stream() };
        let return_type = return_type_to_type(&sig.output);

        match &self.condition_type {
            ContractConditionsType::Requires => quote!(
                kani::assert(#attr, stringify!(#attr_copy));
                #call_to_prior
            ),
            ContractConditionsType::Ensures { argument_names } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);
                quote!(
                    #arg_copies
                    let result: #return_type = #call_to_prior;
                    kani::assume(#attr);
                    #copy_clean
                    result
                )
            }
        }
    }

    /// Emit the check function into the output stream.
    ///
    /// See [`Self::make_check_body`] for the most interesting parts of this
    /// function.
    fn emit_check_function(&mut self, check_function_ident: Ident) {
        self.emit_common_header();

        if self.function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            self.output.extend(quote!(#[kanitool::is_contract_generated(check)]));
        }
        let body = self.make_check_body();
        let mut sig = self.annotated_fn.sig.clone();
        sig.ident = check_function_ident;
        self.output.extend(quote!(
            #sig {
                #body
            }
        ))
    }

    /// Emit the replace funtion into the output stream.
    ///
    /// See [`Self::make_replace_body`] for the most interesting parts of this
    /// function.
    fn emit_replace_function(&mut self, replace_function_ident: Ident, is_first_emit: bool) {
        self.emit_common_header();

        if self.function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            self.output.extend(quote!(#[kanitool::is_contract_generated(replace)]));
        }
        let mut sig = self.annotated_fn.sig.clone();
        if is_first_emit {
            attach_require_kani_any(&mut sig);
        }
        let body = self.make_replace_body(is_first_emit);
        sig.ident = replace_function_ident;

        // Finally emit the check function itself.
        self.output.extend(quote!(
            #sig {
                #body
            }
        ));
    }

    /// Emit attributes common to check or replace function into the output
    /// stream.
    fn emit_common_header(&mut self) {
        if self.function_state.emit_tag_attr() {
            self.output.extend(quote!(
                #[allow(dead_code, unused_variables)]
            ));
        }
        self.output.extend(self.annotated_fn.attrs.iter().flat_map(Attribute::to_token_stream));
    }
}

/// If an explicit return type was provided it is returned, otherwise `()`.
fn return_type_to_type(return_type: &syn::ReturnType) -> Cow<syn::Type> {
    match return_type {
        syn::ReturnType::Default => Cow::Owned(syn::Type::Tuple(syn::TypeTuple {
            paren_token: syn::token::Paren::default(),
            elems: Default::default(),
        })),
        syn::ReturnType::Type(_, typ) => Cow::Borrowed(typ.as_ref()),
    }
}

/// Looks complicated but does something very simple: attach a bound for
/// `kani::Arbitrary` on the return type to the provided signature. Pushes it
/// onto a preexisting where condition, initializing a new `where` condition if
/// it doesn't already exist.
///
/// Very simple example: `fn foo() -> usize { .. }` would be rewritten `fn foo()
/// -> usize where usize: kani::Arbitrary { .. }`.
///
/// This is called when we first emit a replace function. Later we can rely on
/// this bound already being present.
fn attach_require_kani_any(sig: &mut Signature) {
    if matches!(sig.output, ReturnType::Default) {
        // It's the default return type, e.g. `()` so we can skip adding the
        // constraint.
        return;
    }
    let return_ty = return_type_to_type(&sig.output);
    let where_clause = sig.generics.where_clause.get_or_insert_with(|| WhereClause {
        where_token: syn::Token![where](Span::call_site()),
        predicates: Default::default(),
    });

    where_clause.predicates.push(syn::WherePredicate::Type(PredicateType {
        lifetimes: None,
        bounded_ty: return_ty.into_owned(),
        colon_token: syn::Token![:](Span::call_site()),
        bounds: [TypeParamBound::Trait(TraitBound {
            paren_token: None,
            modifier: syn::TraitBoundModifier::None,
            lifetimes: None,
            path: syn::Path {
                leading_colon: None,
                segments: [
                    syn::PathSegment {
                        ident: Ident::new("kani", Span::call_site()),
                        arguments: syn::PathArguments::None,
                    },
                    syn::PathSegment {
                        ident: Ident::new("Arbitrary", Span::call_site()),
                        arguments: syn::PathArguments::None,
                    },
                ]
                .into_iter()
                .collect(),
            },
        })]
        .into_iter()
        .collect(),
    }))
}

/// We make shallow copies of the argument for the postconditions in both
/// `requires` and `ensures` clauses and later clean them up.
///
/// This function creates the code necessary to both make the copies (first
/// tuple elem) and to clean them (second tuple elem).
fn make_unsafe_argument_copies(
    renaming_map: &HashMap<Ident, Ident>,
) -> (TokenStream2, TokenStream2) {
    let arg_names = renaming_map.values();
    let also_arg_names = renaming_map.values();
    let arg_values = renaming_map.keys();
    (
        quote!(#(let #arg_names = kani::untracked_deref(&#arg_values);)*),
        quote!(#(std::mem::forget(#also_arg_names);)*),
    )
}

/// The main meat of handling requires/ensures contracts.
///
/// See the [module level documentation][self] for a description of how the code
/// generation works.
fn requires_ensures_main(attr: TokenStream, item: TokenStream, is_requires: bool) -> TokenStream {
    let attr_copy = TokenStream2::from(attr.clone());
    let attr = parse_macro_input!(attr as Expr);

    let mut output = proc_macro2::TokenStream::new();
    let item_stream_clone = item.clone();
    let item_fn = parse_macro_input!(item as ItemFn);

    let function_state = ContractFunctionState::from_attributes(&item_fn.attrs);

    if matches!(function_state, ContractFunctionState::Original) {
        // If we're the original function that means we're *not* the first time
        // that a contract attribute is handled on this function. This means
        // there must exist a generated check function somewhere onto which the
        // attributes have been copied and where they will be expanded into more
        // checks. So we just return ourselves unchanged.
        //
        // Since this is the only function state case that doesn't need a
        // handler to be constructed, we do this match early, separately.
        return item_fn.into_token_stream().into();
    }

    let mut handler = ContractConditionsHandler::new(
        function_state,
        is_requires,
        attr,
        &item_fn,
        attr_copy,
        &mut output,
    );

    match function_state {
        ContractFunctionState::Check => {
            // The easy cases first: If we are on a check or replace function
            // emit them again but with additional conditions layered on.
            //
            // Since we are already on the check function, it will have an
            // appropriate, unique generated name which we are just going to
            // pass on.
            handler.emit_check_function(item_fn.sig.ident.clone());
        }
        ContractFunctionState::Replace => {
            // Analogous to above
            handler.emit_replace_function(item_fn.sig.ident.clone(), false);
        }
        ContractFunctionState::Original => {
            unreachable!("Impossible: This is handled via short circuiting earlier.")
        }
        ContractFunctionState::Untouched => {
            // The complex case. We are the first time a contract is handled on this function, so
            // we're responsible for
            //
            // 1. Generating a name for the check function
            // 2. Emitting the original, unchanged item and register the check
            //    function on it via attribute
            // 3. Renaming our item to the new name
            // 4. And (minor point) adding #[allow(dead_code)] and
            //    #[allow(unused_variables)] to the check function attributes

            // We'll be using this to postfix the generated names for the "check"
            // and "replace" functions.
            let item_hash = short_hash_of_token_stream(&item_stream_clone);

            let check_fn_name = identifier_for_generated_function(&item_fn, "check", item_hash);
            let replace_fn_name = identifier_for_generated_function(&item_fn, "replace", item_hash);
            let recursion_wrapper_name =
                identifier_for_generated_function(&item_fn, "recursion_wrapper", item_hash);

            // Constructing string literals explicitly here, because `stringify!`
            // doesn't work. Let's say we have an identifier `check_fn` and we were
            // to do `quote!(stringify!(check_fn))` to try to have it expand to
            // `"check_fn"` in the generated code. Then when the next macro parses
            // this it will *not* see the literal `"check_fn"` as you may expect but
            // instead the *expression* `stringify!(check_fn)`.
            let replace_fn_name_str =
                syn::LitStr::new(&replace_fn_name.to_string(), Span::call_site());
            let recursion_wrapper_name_str =
                syn::LitStr::new(&recursion_wrapper_name.to_string(), Span::call_site());

            // The order of `attrs` and `kanitool::{checked_with,
            // is_contract_generated}` is important here, because macros are
            // expanded outside in. This way other contract annotations in `attrs`
            // sees those attributes and can use them to determine
            // `function_state`.
            //
            // The same care is taken when we emit check and replace functions.
            // emit the check function.
            let is_impl_fn = is_probably_impl_fn(&item_fn);
            let ItemFn { attrs, vis, sig, block } = &item_fn;
            handler.output.extend(quote!(
                #(#attrs)*
                #[kanitool::checked_with = #recursion_wrapper_name_str]
                #[kanitool::replaced_with = #replace_fn_name_str]
                #vis #sig {
                    #block
                }
            ));

            let mut wrapper_sig = sig.clone();
            attach_require_kani_any(&mut wrapper_sig);
            wrapper_sig.ident = recursion_wrapper_name;

            let args = pats_to_idents(&mut wrapper_sig.inputs).collect::<Vec<_>>();
            let also_args = args.iter();
            let (call_check, call_replace) = if is_impl_fn {
                (quote!(Self::#check_fn_name), quote!(Self::#replace_fn_name))
            } else {
                (quote!(#check_fn_name), quote!(#replace_fn_name))
            };

            handler.output.extend(quote!(
                #[allow(dead_code, unused_variables)]
                #[kanitool::is_contract_generated(recursion_wrapper)]
                #wrapper_sig {
                    static mut REENTRY: bool = false;
                    if unsafe { REENTRY } {
                        #call_replace(#(#args),*)
                    } else {
                        unsafe { REENTRY = true };
                        let result = #call_check(#(#also_args),*);
                        unsafe { REENTRY = false };
                        result
                    }
                }
            ));

            handler.emit_check_function(check_fn_name);
            handler.emit_replace_function(replace_fn_name, true);
        }
    }

    output.into()
}

/// Convert every use of a pattern in this signature to a simple, fresh, binding-only
/// argument ([`syn::PatIdent`]) and return the [`Ident`] that was generated.
fn pats_to_idents<P>(
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
fn is_probably_impl_fn(fun: &ItemFn) -> bool {
    let mut self_detector = SelfDetector(false);
    self_detector.visit_item_fn(fun);
    self_detector.0
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

passthrough!(stub_verified, false);
passthrough!(proof_for_contract, true);

#[cfg(test)]
mod test {
    macro_rules! detect_impl_fn {
        ($expect_pass:expr, $($tt:tt)*) => {{
            let syntax = stringify!($($tt)*);
            let ast = syn::parse_str(syntax).unwrap();
            assert!($expect_pass == super::is_probably_impl_fn(&ast),
                "Incorrect detection.\nExpected is_impl_fun: {}\nInput Expr; {}\nParsed: {:?}",
                $expect_pass,
                syntax,
                ast
            );
        }}
    }

    #[test]
    fn detect_impl_fn_by_receiver() {
        detect_impl_fn!(true, fn self_by_ref(&self, u: usize) -> bool {});

        detect_impl_fn!(true, fn self_by_self(self, u: usize) -> bool {});
    }

    #[test]
    fn detect_impl_fn_by_self_ty() {
        detect_impl_fn!(true, fn self_by_construct(u: usize) -> Self {});
        detect_impl_fn!(true, fn self_by_wrapped_construct(u: usize) -> Arc<Self> {});

        detect_impl_fn!(true, fn self_by_other_arg(u: usize, slf: Self) {});

        detect_impl_fn!(true, fn self_by_other_wrapped_arg(u: usize, slf: Vec<Self>) {})
    }

    #[test]
    fn detect_impl_fn_by_qself() {
        detect_impl_fn!(
            true,
            fn self_by_mention(u: usize) {
                Self::other(u)
            }
        );
    }

    #[test]
    fn detect_no_impl_fn() {
        detect_impl_fn!(
            false,
            fn self_by_mention(u: usize) {
                let self_name = 18;
                let self_lit = "self";
                let self_lit = "Self";
            }
        );
    }
}
