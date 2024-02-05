// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Logic used for generating the code that checks a contract.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Expr, FnArg, ItemFn, Token};

use crate::sysroot::contracts::shared::create_history_expr_bindings;

use super::{
    helpers::*,
    shared::{make_unsafe_argument_copies, try_as_result_assign_mut},
    ContractConditionsData, ContractConditionsHandler,
};

impl<'a> ContractConditionsHandler<'a> {
    /// Create the body of a check function.
    ///
    /// Wraps the conditions from this attribute around `self.body`.
    ///
    /// Mutable because a `modifies` clause may need to extend the inner call to
    /// the wrapper with new arguments.
    pub fn make_check_body(&mut self) -> TokenStream2 {
        let mut inner = self.ensure_bootstrapped_check_body();
        let Self { attr_copy, .. } = self;

        match &self.condition_type {
            ContractConditionsData::Requires { attr } => {
                quote!(
                    kani::assume(#attr);
                    #(#inner)*
                )
            }
            ContractConditionsData::Ensures { argument_names, attr, history_expressions } => {
                let (arg_copies, copy_clean) = make_unsafe_argument_copies(&argument_names);
                let history_bindings = create_history_expr_bindings(history_expressions);

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
                    #history_bindings
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

    /// Emit the check function into the output stream.
    ///
    /// See [`Self::make_check_body`] for the most interesting parts of this
    /// function.
    pub fn emit_check_function(&mut self, override_function_dent: Option<Ident>) {
        self.emit_common_header();

        if self.function_state.emit_tag_attr() {
            // If it's the first time we also emit this marker. Again, order is
            // important so this happens as the last emitted attribute.
            self.output.extend(quote!(#[kanitool::is_contract_generated(check)]));
        }
        let body = self.make_check_body();
        let mut sig = self.annotated_fn.sig.clone();
        if let Some(ident) = override_function_dent {
            sig.ident = ident;
        }
        self.output.extend(quote!(
            #sig {
                #body
            }
        ))
    }

    /// Emit a modifies wrapper, possibly augmenting a prior, existing one.
    ///
    /// We only augment if this clause is a `modifies` clause. In that case we
    /// expand its signature with one new argument of type `&impl Arbitrary` for
    /// each expression in the clause.
    pub fn emit_augmented_modifies_wrapper(&mut self) {
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
