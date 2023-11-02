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
//!                            v
//!                      +-----------+
//!                      | Untouched |
//!                      | Function  |
//!                      +-----+-----+
//!                            |
//!             Emit           |  Generate + Copy Attributes
//!          +-----------------+------------------+
//!          |                 |                  |
//!          |                 |                  |
//!          v                 v                  v
//!   +----------+           +-------+        +---------+
//!   | Original |<-+        | Check |<-+     | Replace |<-+
//!   +--+-------+  |        +---+---+  |     +----+----+  |
//!      |          | Ignore     |      | Augment  |       | Augment
//!      +----------+            +------+          +-------+
//!
//! |              |       |                              |
//! +--------------+       +------------------------------+
//!   Presence of                     Presence of
//!  "checked_with"             "is_contract_generated"
//!
//!            State is detected via
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
//! Generates a `check_<fn_name>_<fn_hash>` function that assumes preconditions
//! and asserts postconditions. The check function is also marked as generated
//! with the `#[kanitool::is_contract_generated(check)]` attribute.
//!
//! Decorates the original function with `#[kanitool::checked_by =
//! "check_<fn_name>_<fn_hash>"]`.
//!
//! The check function is a copy of the original function with preconditions
//! added before the body and postconditions after as well as injected before
//! every `return` (see [`PostconditionInjector`]). Attributes on the original
//! function are also copied to the check function.
//!
//! ## Replace Function
//!
//! As the mirror to that also generates a `replace_<fn_name>_<fn_hash>`
//! function that asserts preconditions and assumes postconditions. The replace
//! function is also marked as generated with the
//! `#[kanitool::is_contract_generated(replace)]` attribute.
//!
//! Decorates the original function with `#[kanitool::replaced_by =
//! "replace_<fn_name>_<fn_hash>"]`.
//!
//! The replace function has the same signature as the original function but its
//! body is replaced by `kani::any()`, which generates a non-deterministic
//! value.
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
//! #[kanitool::checked_with = "div_check_965916"]
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
    GenericArgument, ItemFn, PredicateType, ReturnType, Signature, Token, TraitBound,
    TypeParamBound, WhereClause,
};

pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_main(attr, item, 0)
}

pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_main(attr, item, 1)
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
    condition_type: ContractConditionsType,
    /// Body of the function this attribute was found on.
    annotated_fn: &'a mut ItemFn,
    /// An unparsed, unmodified copy of `attr`, used in the error messages.
    attr_copy: TokenStream2,
    /// The stream to which we should write the generated code.
    output: &'a mut TokenStream2,
    hash: Option<u64>,
}

/// Information needed for generating check and replace handlers for different
/// contract attributes.
enum ContractConditionsType {
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

impl ContractConditionsType {
    /// Constructs a [`Self::Ensures`] from the signature of the decorated
    /// function and the contents of the decorating attribute.
    ///
    /// Renames the [`Ident`]s used in `attr` and stores the translation map in
    /// `argument_names`.
    fn new_ensures(sig: &Signature, mut attr: Expr) -> Self {
        let argument_names = rename_argument_occurrences(sig, &mut attr);
        ContractConditionsType::Ensures { argument_names, attr }
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
        is_requires: u8,
        attr: TokenStream,
        annotated_fn: &'a mut ItemFn,
        attr_copy: TokenStream2,
        output: &'a mut TokenStream2,
        hash: Option<u64>,
    ) -> Result<Self, syn::Error> {
        let condition_type = match is_requires {
            0 => ContractConditionsType::Requires { attr: syn::parse(attr)? },
            1 => ContractConditionsType::new_ensures(&annotated_fn.sig, syn::parse(attr)?),
            2 => ContractConditionsType::Modifies {
                attr: chunks_by(TokenStream2::from(attr), is_token_stream_2_comma)
                    .map(syn::parse2)
                    .filter_map(|expr| match expr {
                        Err(e) => {
                            output.extend(e.into_compile_error());
                            None
                        }
                        Ok(expr) => Some(expr),
                    })
                    .collect(),
            },
            _ => unreachable!(),
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
        let Self { attr_copy, .. } = self;

        match &self.condition_type {
            ContractConditionsType::Requires { attr } => {
                let block = self.create_inner_call([].into_iter());
                quote!(
                    kani::assume(#attr);
                    #(#block)*
                )
            }
            ContractConditionsType::Ensures { argument_names, attr } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);

                // The code that enforces the postconditions and cleans up the shallow
                // argument copies (with `mem::forget`).
                let exec_postconditions = quote!(
                    kani::assert(#attr, stringify!(#attr_copy));
                    #copy_clean
                );

                let mut call = self.create_inner_call([].into_iter());

                assert!(
                    matches!(call.pop(), Some(syn::Stmt::Expr(syn::Expr::Path(pexpr), None)) if pexpr.path.get_ident().map_or(false, |id| id == "result"))
                );

                quote!(
                    #arg_copies
                    #(#call)*
                    #exec_postconditions
                    result
                )
            }
            ContractConditionsType::Modifies { attr } => {
                let wrapper_name = self.make_wrapper_name().to_string();
                let wrapper_args = make_wrapper_args(attr.len());
                // TODO handle first invocation where this is the actual body.
                if !self.is_first_emit() {
                    if let Some(wrapper_call_args) = self
                        .annotated_fn
                        .block
                        .stmts
                        .iter_mut()
                        .find_map(|stmt| try_as_wrapper_call_args(stmt, &wrapper_name))
                    {
                        wrapper_call_args
                            .extend(wrapper_args.clone().map(|a| Expr::Verbatim(quote!(#a))));
                    } else {
                        unreachable!(
                            "Invariant broken, check function did not contain a call to the wrapper function"
                        )
                    }
                }

                let inner = self.create_inner_call(wrapper_args.clone());
                let wrapper_args = make_wrapper_args(attr.len());

                quote!(
                    #(let #wrapper_args = unsafe { kani::DecoupleLifetime::decouple_lifetime(&#attr) };)*
                    #(#inner)*
                )
            }
        }
    }

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

    fn create_inner_call(&self, additional_args: impl Iterator<Item = Ident>) -> Vec<syn::Stmt> {
        let wrapper_name = self.make_wrapper_name();
        let return_type = return_type_to_type(&self.annotated_fn.sig.output);
        if self.is_first_emit() {
            let args = exprs_for_args(&self.annotated_fn.sig.inputs);
            syn::parse_quote!(
                let result : #return_type = #wrapper_name(#(#args,)* #(#additional_args),*);
                result
            )
        } else {
            self.annotated_fn.block.stmts.clone()
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
        let Self { attr_copy, .. } = self;
        let ItemFn { sig, block, .. } = &*self.annotated_fn;
        let call_to_prior =
            if use_nondet_result { quote!(kani::any()) } else { block.to_token_stream() };
        let return_type = return_type_to_type(&sig.output);

        match &self.condition_type {
            ContractConditionsType::Requires { attr } => quote!(
                kani::assert(#attr, stringify!(#attr_copy));
                #call_to_prior
            ),
            ContractConditionsType::Ensures { attr, argument_names } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);
                quote!(
                    #arg_copies
                    let result: #return_type = #call_to_prior;
                    kani::assume(#attr);
                    #copy_clean
                    result
                )
            }
            ContractConditionsType::Modifies { .. } => {
                quote!(kani::assert(false, "Replacement with modifies is not supported yet."))
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

    fn emit_augmented_modifies_wrapper(&mut self) {
        if let ContractConditionsType::Modifies { attr } = &self.condition_type {
            let wrapper_args = make_wrapper_args(attr.len());
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

fn exprs_for_args<'a, T>(
    args: &'a syn::punctuated::Punctuated<FnArg, T>,
) -> impl Iterator<Item = Expr> + Clone + 'a {
    args.iter().map(|arg| match arg {
        FnArg::Receiver(_) => Expr::Verbatim(quote!(self)),
        FnArg::Typed(typed) => pat_to_expr(&typed.pat),
    })
}

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
        Pat::Type(_) => mk_err("type"),
        Pat::TupleStruct(_) => mk_err("tuple struct"),
        _ => mk_err("unknown"),
    }
}

fn try_as_wrapper_call_args<'a>(
    stmt: &'a mut syn::Stmt,
    wrapper_fn_name: &str,
) -> Option<&'a mut syn::punctuated::Punctuated<syn::Expr, syn::token::Comma>> {
    match stmt {
        syn::Stmt::Local(syn::Local {
            pat: syn::Pat::Type(syn::PatType { pat: inner_pat, .. }),
            init: Some(syn::LocalInit { diverge: None, expr: init_expr, .. }),
            ..
        }) if matches!(inner_pat.as_ref(),
          syn::Pat::Ident(syn::PatIdent {
                        by_ref: None,
                        mutability: None,
                        ident: result_ident,
                        subpat: None,
                        ..
                    }) if result_ident == "result"
        ) =>
        {
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
        _ => None,
    }
}

fn make_wrapper_args(num: usize) -> impl Iterator<Item = syn::Ident> + Clone {
    (0..num).map(|i| Ident::new(&format!("_wrapper_arg_{i}"), Span::mixed_site()))
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
fn requires_ensures_main(attr: TokenStream, item: TokenStream, is_requires: u8) -> TokenStream {
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
            handler.emit_replace_function(original_function_name, false);
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

            // Constructing string literals explicitly here, because `stringify!`
            // doesn't work. Let's say we have an identifier `check_fn` and we were
            // to do `quote!(stringify!(check_fn))` to try to have it expand to
            // `"check_fn"` in the generated code. Then when the next macro parses
            // this it will *not* see the literal `"check_fn"` as you may expect but
            // instead the *expression* `stringify!(check_fn)`.
            let replace_fn_name_str =
                syn::LitStr::new(&replace_fn_name.to_string(), Span::call_site());
            let check_fn_name_str = syn::LitStr::new(&check_fn_name.to_string(), Span::call_site());
            let wrapper_fn_name_str =
                syn::LitStr::new(&handler.make_wrapper_name().to_string(), Span::call_site());

            // The order of `attrs` and `kanitool::{checked_with,
            // is_contract_generated}` is important here, because macros are
            // expanded outside in. This way other contract annotations in `attrs`
            // sees those attributes and can use them to determine
            // `function_state`.
            //
            // The same care is taken when we emit check and replace functions.
            // emit the check function.
            let ItemFn { attrs, vis, sig, block } = &handler.annotated_fn;
            let reemit_tokens = quote!(
                #(#attrs)*
                #[kanitool::checked_with = #check_fn_name_str]
                #[kanitool::replaced_with = #replace_fn_name_str]
                #[kanitool::inner_check = #wrapper_fn_name_str]
                #vis #sig {
                    #block
                }
            );
            handler.output.extend(reemit_tokens);

            handler.emit_check_function(check_fn_name);
            handler.emit_replace_function(replace_fn_name, true);
            handler.emit_augmented_modifies_wrapper();
        }
    }

    output.into()
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
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::default();
    hash_of_token_stream(&mut hasher, proc_macro2::TokenStream::from(stream.clone()));
    let long_hash = hasher.finish();
    long_hash % 0x1_000_000 // six hex digits
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

#[allow(dead_code)]
pub fn modifies(attr: TokenStream, item: TokenStream) -> TokenStream {
    requires_ensures_main(attr, item, 2)
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
        while let Some(tok) = iter.next() {
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
