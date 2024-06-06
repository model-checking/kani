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
//!         let result_kani_internal = call_check(fn args...);
//!         unsafe { reentry = false };
//!         result_kani_internal
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
//! #[kani::ensures(|result : &u32| *result <= dividend)]
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
//! #[allow(dead_code, unused_variables)]
//! #[kanitool :: is_contract_generated(check)] fn
//! div_check_b97df2(dividend : u32, divisor : u32) -> u32
//! {
//!     let dividend_renamed = kani::internal::untracked_deref(& dividend);
//!     let divisor_renamed = kani::internal::untracked_deref(& divisor);
//!     kani::assume(divisor != 0);
//!     let result_kani_internal : u32 = div_wrapper_b97df2(dividend, divisor);
//!     kani::assert(
//!     (| result : & u32 | *result <= dividend_renamed)(& result_kani_internal),
//!     stringify!(|result : &u32| *result <= dividend));
//!     std::mem::forget(dividend_renamed);
//!     std::mem::forget(divisor_renamed);
//!     result_kani_internal
//! }
//!
//! #[allow(dead_code, unused_variables)]
//! #[kanitool :: is_contract_generated(replace)] fn
//! div_replace_b97df2(dividend : u32, divisor : u32) -> u32
//! {
//!     let divisor_renamed = kani::internal::untracked_deref(& divisor);
//!     let dividend_renamed = kani::internal::untracked_deref(& dividend);
//!     kani::assert(divisor != 0, stringify! (divisor != 0));
//!     let result_kani_internal : u32 = kani::any_modifies();
//!     kani::assume(
//!     (|result : & u32| *result <= dividend_renamed)(&result_kani_internal));
//!     std::mem::forget(divisor_renamed);
//!     std::mem::forget(dividend_renamed);
//!     result_kani_internal
//! }
//!
//! #[allow(dead_code)]
//! #[allow(unused_variables)]
//! #[kanitool::is_contract_generated(recursion_wrapper)]
//! fn div_recursion_wrapper_965916(dividend: u32, divisor: u32) -> u32 {
//!     static mut REENTRY: bool = false;
//!
//!     if unsafe { REENTRY } {
//!         div_replace_b97df2(dividend, divisor)
//!     } else {
//!         unsafe { reentry = true };
//!         let result_kani_internal = div_check_b97df2(dividend, divisor);
//!         unsafe { reentry = false };
//!         result_kani_internal
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::collections::HashMap;
use syn::{parse_macro_input, Expr, ExprClosure, ItemFn};

mod bootstrap;
mod check;
#[macro_use]
mod helpers;
mod initialize;
mod replace;
mod shared;

const INTERNAL_RESULT_IDENT: &str = "result_kani_internal";

pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    contract_main(attr, item, ContractConditionsType::Requires)
}

pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    contract_main(attr, item, ContractConditionsType::Ensures)
}

pub fn modifies(attr: TokenStream, item: TokenStream) -> TokenStream {
    contract_main(attr, item, ContractConditionsType::Modifies)
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
    output: TokenStream2,
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
        attr: ExprClosure,
    },
    Modifies {
        attr: Vec<Expr>,
    },
}

impl<'a> ContractConditionsHandler<'a> {
    /// Handle the contract state and return the generated code
    fn dispatch_on(mut self, state: ContractFunctionState) -> TokenStream2 {
        match state {
            ContractFunctionState::ModifiesWrapper => self.emit_augmented_modifies_wrapper(),
            ContractFunctionState::Check => {
                // The easy cases first: If we are on a check or replace function
                // emit them again but with additional conditions layered on.
                //
                // Since we are already on the check function, it will have an
                // appropriate, unique generated name which we are just going to
                // pass on.
                self.emit_check_function(None);
            }
            ContractFunctionState::Replace => {
                // Analogous to above
                self.emit_replace_function(None);
            }
            ContractFunctionState::Original => {
                unreachable!("Impossible: This is handled via short circuiting earlier.")
            }
            ContractFunctionState::Untouched => self.handle_untouched(),
        }
        self.output
    }
}

/// The main meat of handling requires/ensures contracts.
///
/// See the [module level documentation][self] for a description of how the code
/// generation works.
fn contract_main(
    attr: TokenStream,
    item: TokenStream,
    is_requires: ContractConditionsType,
) -> TokenStream {
    let attr_copy = TokenStream2::from(attr.clone());

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
        .then(|| helpers::short_hash_of_token_stream(&item_stream_clone));

    let handler = match ContractConditionsHandler::new(
        function_state,
        is_requires,
        attr,
        &mut item_fn,
        attr_copy,
        hash,
    ) {
        Ok(handler) => handler,
        Err(e) => return e.into_compile_error().into(),
    };

    handler.dispatch_on(function_state).into()
}
