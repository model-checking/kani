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
//! with the `kani::internal::untracked_deref` function to circumvent the borrow checker
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
//!     let dividend_renamed = kani::internal::untracked_deref(&dividend);
//!     let divisor_renamed = kani::internal::untracked_deref(&divisor);
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
//!     let dividend_renamed = kani::internal::untracked_deref(&dividend);
//!     let divisor_renamed = kani::internal::untracked_deref(&divisor);
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
    parse_macro_input, spanned::Spanned, visit::Visit, visit_mut::VisitMut, Attribute, Expr, FnArg,
    ItemFn, PredicateType, ReturnType, Signature, Token, TraitBound, TypeParamBound, WhereClause,
};

#[allow(dead_code)]
pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_main(attr, item, ContractConditionsType::Requires)
}

#[allow(dead_code)]
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_main(attr, item, ContractConditionsType::Ensures)
}

#[allow(dead_code)]
pub fn modifies(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_main(attr, item, ContractConditionsType::Modifies)
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

pub fn proof_for_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(attr);
    let ItemFn { attrs, vis, sig, block } = parse_macro_input!(item as ItemFn);
    quote!(
        #[allow(dead_code)]
        #[kanitool::proof_for_contract = stringify!(#args)]
        #(#attrs)*
        #vis #sig {
            let _ = std::boxed::Box::new(0_usize);
            #block
        }
    )
    .into()
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
    ModifiesWrapper,
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

/// The information needed to generate the bodies of check and replacement
/// functions that integrate the conditions from this contract attribute.
struct ContractConditionsHandler<'a> {
    function_state: ContractFunctionState,
    /// Information specific to the type of contract attribute we're expanding.
    condition_type: ContractConditionsData,
    /// Body of the function this attribute was found on.
    annotated_fn: &'a mut ItemFn,
    /// An unparsed, unmodified copy of `attr`, used in the error messages.
    attr_copy: TokenStream2,
    /// The stream to which we should write the generated code.
    output: &'a mut TokenStream2,
    hash: Option<u64>,
}

/// Which kind of contract attribute are we dealing with?
///
/// Pre-parsing version of [`ContractConditionsData`].
#[derive(Copy, Clone, Eq, PartialEq)]
enum ContractConditionsType {
    Requires,
    Ensures,
    Modifies,
}

/// Clause-specific information mostly generated by parsing the attribute.
///
/// [`ContractConditionsType`] is the corresponding pre-parse version.
enum ContractConditionsData {
    Requires {
        /// The contents of the attribute.
        attr: Expr,
    },
    Ensures {
        /// Translation map from original argument names to names of the copies
        /// we will be emitting.
        argument_names: HashMap<Ident, Ident>,
        /// The contents of the attribute.
        attr: Expr,
    },
    Modifies {
        attr: Vec<Expr>,
    },
}

impl ContractConditionsData {
    /// Constructs a [`Self::Ensures`] from the signature of the decorated
    /// function and the contents of the decorating attribute.
    ///
    /// Renames the [`Ident`]s used in `attr` and stores the translation map in
    /// `argument_names`.
    fn new_ensures(sig: &Signature, mut attr: Expr) -> Self {
        let argument_names = rename_argument_occurrences(sig, &mut attr);
        ContractConditionsData::Ensures { argument_names, attr }
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

impl<'a> ContractConditionsHandler<'a> {
    fn is_first_emit(&self) -> bool {
        matches!(self.function_state, ContractFunctionState::Untouched)
    }

    /// Initialize the handler. Constructs the required
    /// [`ContractConditionsType`] depending on `is_requires`.
    fn new(
        function_state: ContractFunctionState,
        is_requires: ContractConditionsType,
        attr: TokenStream,
        annotated_fn: &'a mut ItemFn,
        attr_copy: TokenStream2,
        output: &'a mut TokenStream2,
        hash: Option<u64>,
    ) -> Result<Self, syn::Error> {
        let condition_type = match is_requires {
            ContractConditionsType::Requires => {
                ContractConditionsData::Requires { attr: syn::parse(attr)? }
            }
            ContractConditionsType::Ensures => {
                ContractConditionsData::new_ensures(&annotated_fn.sig, syn::parse(attr)?)
            }
            ContractConditionsType::Modifies => ContractConditionsData::new_modifies(attr, output),
        };

        Ok(Self { function_state, condition_type, annotated_fn, attr_copy, output, hash })
    }

    /// Create the body of a check function.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    ///
    /// Mutable because a `modifies` clause may need to extend the inner call to
    /// the wrapper with new arguments.
    fn make_check_body(&mut self) -> TokenStream2 {
        let mut inner = self.ensure_bootstrapped_check_body();
        let Self { attr_copy, .. } = self;

        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                quote!(
                    kani::assume(#attr);
                    #(#inner)*
                )
            }
            ContractConditionsData::Ensures { argument_names, attr } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);

                // The code that enforces the postconditions and cleans up the shallow
                // argument copies (with `mem::forget`).
                let exec_postconditions = quote!(
                    kani::assert(#attr, stringify!(#attr_copy));
                    #copy_clean
                );

                assert!(matches!(
                    inner.pop(),
                    Some(syn::Stmt::Expr(syn::Expr::Path(pexpr), None))
                        if pexpr.path.get_ident().map_or(false, |id| id == "result")
                ));

                quote!(
                    #arg_copies
                    #(#inner)*
                    #exec_postconditions
                    result
                )
            }
            ContractConditionsData::Modifies { attr } => {
                let wrapper_name = self.make_wrapper_name().to_string();

                let wrapper_args = if let Some(wrapper_call_args) =
                    inner.iter_mut().find_map(|stmt| try_as_wrapper_call_args(stmt, &wrapper_name))
                {
                    let wrapper_args = make_wrapper_args(wrapper_call_args.len(), attr.len());
                    wrapper_call_args
                        .extend(wrapper_args.clone().map(|a| Expr::Verbatim(quote!(#a))));
                    wrapper_args
                } else {
                    unreachable!(
                        "Invariant broken, check function did not contain a call to the wrapper function"
                    )
                };

                quote!(
                    #(let #wrapper_args = unsafe { kani::internal::Pointer::decouple_lifetime(&#attr) };)*
                    #(#inner)*
                )
            }
        }
    }

    /// Create a new name for the assigns wrapper function *or* get the name of
    /// the wrapper we must have already generated. This is so that we can
    /// recognize a call to that wrapper inside the check function.
    fn make_wrapper_name(&self) -> Ident {
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

    /// Get the sequence of statements of the previous check body or create the default one.
    fn ensure_bootstrapped_check_body(&self) -> Vec<syn::Stmt> {
        let wrapper_name = self.make_wrapper_name();
        let return_type = return_type_to_type(&self.annotated_fn.sig.output);
        if self.is_first_emit() {
            let args = exprs_for_args(&self.annotated_fn.sig.inputs);
            let wrapper_call = if is_probably_impl_fn(self.annotated_fn) {
                quote!(Self::#wrapper_name)
            } else {
                quote!(#wrapper_name)
            };
            syn::parse_quote!(
                let result : #return_type = #wrapper_call(#(#args),*);
                result
            )
        } else {
            self.annotated_fn.block.stmts.clone()
        }
    }

    /// Split an existing replace body of the form
    ///
    /// ```ignore
    /// // multiple preconditions and argument copies like like
    /// kani::assert(.. precondition);
    /// let arg_name = kani::internal::untracked_deref(&arg_value);
    /// // single result havoc
    /// let result : ResultType = kani::any();
    ///
    /// // multiple argument havockings
    /// *unsafe { kani::internal::Pointer::assignable(argument) } = kani::any();
    /// // multiple postconditions
    /// kani::assume(postcond);
    /// // multiple argument copy (used in postconditions) cleanups
    /// std::mem::forget(arg_name);
    /// // single return
    /// result
    /// ```
    ///
    /// Such that the first vector contains everything up to and including the single result havoc
    /// and the second one the rest, excluding the return.
    ///
    /// If this is the first time we're emitting replace we create the return havoc and nothing else.
    fn ensure_bootstrapped_replace_body(&self) -> (Vec<syn::Stmt>, Vec<syn::Stmt>) {
        if self.is_first_emit() {
            let return_type = return_type_to_type(&self.annotated_fn.sig.output);
            (vec![syn::parse_quote!(let result : #return_type = kani::any();)], vec![])
        } else {
            let stmts = &self.annotated_fn.block.stmts;
            let idx = stmts
                .iter()
                .enumerate()
                .find_map(|(i, elem)| is_replace_return_havoc(elem).then_some(i))
                .unwrap_or_else(|| {
                    panic!("ICE: Could not find result let binding in statement sequence")
                });
            // We want the result assign statement to end up as the last statement in the first
            // vector, hence the `+1`.
            let (before, after) = stmts.split_at(idx + 1);
            (before.to_vec(), after.split_last().unwrap().1.to_vec())
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
    fn make_replace_body(&self) -> TokenStream2 {
        let (before, after) = self.ensure_bootstrapped_replace_body();

        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                let Self { attr_copy, .. } = self;
                quote!(
                    kani::assert(#attr, stringify!(#attr_copy));
                    #(#before)*
                    #(#after)*
                    result
                )
            }
            ContractConditionsData::Ensures { attr, argument_names } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);
                quote!(
                    #arg_copies
                    #(#before)*
                    #(#after)*
                    kani::assume(#attr);
                    #copy_clean
                    result
                )
            }
            ContractConditionsData::Modifies { attr } => {
                quote!(
                    #(#before)*
                    #(*unsafe { kani::internal::Pointer::assignable(#attr) } = kani::any();)*
                    #(#after)*
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
    fn emit_replace_function(&mut self, replace_function_ident: Ident) {
        self.emit_common_header();

        if self.function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            self.output.extend(quote!(#[kanitool::is_contract_generated(replace)]));
        }
        let mut sig = self.annotated_fn.sig.clone();
        if self.is_first_emit() {
            attach_require_kani_any(&mut sig);
        }
        let body = self.make_replace_body();
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

    /// Emit a modifies wrapper, possibly augmenting a prior, existing one.
    ///
    /// We only augment if this clause is a `modifies` clause. In that case we
    /// expand its signature with one new argument of type `&impl Arbitrary` for
    /// each expression in the clause.
    fn emit_augmented_modifies_wrapper(&mut self) {
        if let ContractConditionsData::Modifies { attr } = &self.condition_type {
            let wrapper_args = make_wrapper_args(self.annotated_fn.sig.inputs.len(), attr.len());
            let sig = &mut self.annotated_fn.sig;
            for arg in wrapper_args.clone() {
                let lifetime = syn::Lifetime { apostrophe: Span::call_site(), ident: arg.clone() };
                sig.inputs.push(FnArg::Typed(syn::PatType {
                    attrs: vec![],
                    colon_token: Token![:](Span::call_site()),
                    pat: Box::new(syn::Pat::Verbatim(quote!(#arg))),
                    ty: Box::new(syn::Type::Verbatim(quote!(&#lifetime impl kani::Arbitrary))),
                }));
                sig.generics.params.push(syn::GenericParam::Lifetime(syn::LifetimeParam {
                    lifetime,
                    colon_token: None,
                    bounds: Default::default(),
                    attrs: vec![],
                }));
            }
            self.output.extend(quote!(#[kanitool::modifies(#(#wrapper_args),*)]))
        }
        self.emit_common_header();

        if self.function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            self.output.extend(quote!(#[kanitool::is_contract_generated(wrapper)]));
        }

        let name = self.make_wrapper_name();
        let ItemFn { vis, sig, block, .. } = self.annotated_fn;

        let mut sig = sig.clone();
        sig.ident = name;
        self.output.extend(quote!(
            #vis #sig #block
        ));
    }
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
                }) if result_ident == "result"
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
fn try_as_result_assign(stmt: &syn::Stmt) -> Option<&syn::LocalInit> {
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
fn try_as_result_assign_mut(stmt: &mut syn::Stmt) -> Option<&mut syn::LocalInit> {
    try_as_result_assign_pat!(stmt, as_mut)
}

/// Is this statement `let result : <...> = kani::any();`.
fn is_replace_return_havoc(stmt: &syn::Stmt) -> bool {
    let Some(syn::LocalInit { diverge: None, expr: e, .. }) = try_as_result_assign(stmt) else {
        return false;
    };

    matches!(
        e.as_ref(),
        Expr::Call(syn::ExprCall {
            func,
            args,
            ..
        })
        if args.is_empty()
        && matches!(
            func.as_ref(),
            Expr::Path(syn::ExprPath {
                qself: None,
                path,
                attrs,
            })
            if path.segments.len() == 2
            && path.segments[0].ident == "kani"
            && path.segments[1].ident == "any"
            && attrs.is_empty()
        )
    )
}

/// For each argument create an expression that passes this argument along unmodified.
///
/// Reconstructs structs that may have been deconstructed with patterns.
fn exprs_for_args<T>(
    args: &syn::punctuated::Punctuated<FnArg, T>,
) -> impl Iterator<Item = Expr> + Clone + '_ {
    args.iter().map(|arg| match arg {
        FnArg::Receiver(_) => Expr::Verbatim(quote!(self)),
        FnArg::Typed(typed) => pat_to_expr(&typed.pat),
    })
}

/// Create an expression that reconstructs a struct that was matched in a pattern.
///
/// Does not support enums, wildcards, pattern alternatives (`|`), range patterns, or verbatim.
fn pat_to_expr(pat: &syn::Pat) -> Expr {
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

/// Try to interpret this statement as `let result : <...> = <wrapper_fn_name>(args ...);` and
/// return a mutable reference to the parameter list.
fn try_as_wrapper_call_args<'a>(
    stmt: &'a mut syn::Stmt,
    wrapper_fn_name: &str,
) -> Option<&'a mut syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>> {
    let syn::LocalInit { diverge: None, expr: init_expr, .. } = try_as_result_assign_mut(stmt)?
    else {
        return None;
    };

    match init_expr.as_mut() {
        Expr::Call(syn::ExprCall { func: box_func, args, .. }) => match box_func.as_ref() {
            syn::Expr::Path(syn::ExprPath { qself: None, path, .. })
                if path.get_ident().map_or(false, |id| id == wrapper_fn_name) =>
            {
                Some(args)
            }
            _ => None,
        },
        _ => None,
    }
}

/// Make `num` [`Ident`]s with the names `_wrapper_arg_{i}` with `i` starting at `low` and
/// increasing by one each time.
fn make_wrapper_args(low: usize, num: usize) -> impl Iterator<Item = syn::Ident> + Clone {
    (low..).map(|i| Ident::new(&format!("_wrapper_arg_{i}"), Span::mixed_site())).take(num)
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
        quote!(#(let #arg_names = kani::internal::untracked_deref(&#arg_values);)*),
        quote!(#(std::mem::forget(#also_arg_names);)*),
    )
}

/// The main meat of handling requires/ensures contracts.
///
/// See the [module level documentation][self] for a description of how the code
/// generation works.
fn requires_ensures_main(
    attr: TokenStream,
    item: TokenStream,
    is_requires: ContractConditionsType,
) -> TokenStream {
    let attr_copy = TokenStream2::from(attr.clone());

    let mut output = proc_macro2::TokenStream::new();
    let item_stream_clone = item.clone();
    let mut item_fn = parse_macro_input!(item as ItemFn);

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

    let hash = matches!(function_state, ContractFunctionState::Untouched)
        .then(|| short_hash_of_token_stream(&item_stream_clone));

    let original_function_name = item_fn.sig.ident.clone();

    let mut handler = match ContractConditionsHandler::new(
        function_state,
        is_requires,
        attr,
        &mut item_fn,
        attr_copy,
        &mut output,
        hash,
    ) {
        Ok(handler) => handler,
        Err(e) => return e.into_compile_error().into(),
    };

    match function_state {
        ContractFunctionState::ModifiesWrapper => handler.emit_augmented_modifies_wrapper(),
        ContractFunctionState::Check => {
            // The easy cases first: If we are on a check or replace function
            // emit them again but with additional conditions layered on.
            //
            // Since we are already on the check function, it will have an
            // appropriate, unique generated name which we are just going to
            // pass on.
            handler.emit_check_function(original_function_name);
        }
        ContractFunctionState::Replace => {
            // Analogous to above
            handler.emit_replace_function(original_function_name);
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
            let item_hash = hash.unwrap();

            let check_fn_name =
                identifier_for_generated_function(&original_function_name, "check", item_hash);
            let replace_fn_name =
                identifier_for_generated_function(&original_function_name, "replace", item_hash);
            let recursion_wrapper_name = identifier_for_generated_function(
                &original_function_name,
                "recursion_wrapper",
                item_hash,
            );

            // Constructing string literals explicitly here, because `stringify!`
            // doesn't work. Let's say we have an identifier `check_fn` and we were
            // to do `quote!(stringify!(check_fn))` to try to have it expand to
            // `"check_fn"` in the generated code. Then when the next macro parses
            // this it will *not* see the literal `"check_fn"` as you may expect but
            // instead the *expression* `stringify!(check_fn)`.
            let replace_fn_name_str =
                syn::LitStr::new(&replace_fn_name.to_string(), Span::call_site());
            let wrapper_fn_name_str =
                syn::LitStr::new(&handler.make_wrapper_name().to_string(), Span::call_site());
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
            let is_impl_fn = is_probably_impl_fn(&handler.annotated_fn);
            let ItemFn { attrs, vis, sig, block } = &handler.annotated_fn;
            handler.output.extend(quote!(
                #(#attrs)*
                #[kanitool::checked_with = #recursion_wrapper_name_str]
                #[kanitool::replaced_with = #replace_fn_name_str]
                #[kanitool::inner_check = #wrapper_fn_name_str]
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
            handler.emit_replace_function(replace_fn_name);
            handler.emit_augmented_modifies_wrapper();
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
    const SIX_HEX_DIGITS_MASK: u64 = 0x1_000_000;
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    hash_of_token_stream(&mut hasher, proc_macro2::TokenStream::from(stream.clone()));
    let long_hash = hasher.finish();
    long_hash % SIX_HEX_DIGITS_MASK
}

/// Makes consistent names for a generated function which was created for
/// `purpose`, from an attribute that decorates `related_function` with the
/// hash `hash`.
fn identifier_for_generated_function(
    related_function_name: &Ident,
    purpose: &str,
    hash: u64,
) -> Ident {
    let identifier = format!("{}_{purpose}_{hash:x}", related_function_name);
    Ident::new(&identifier, proc_macro2::Span::mixed_site())
}

fn is_token_stream_2_comma(t: &proc_macro2::TokenTree) -> bool {
    matches!(t, proc_macro2::TokenTree::Punct(p) if p.as_char() == ',')
}

fn chunks_by<'a, T, C: Default + Extend<T>>(
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
