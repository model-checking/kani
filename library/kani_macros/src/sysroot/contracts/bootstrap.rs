// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Special way we handle the first time we encounter a contract attribute on a
//! function.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{Expr, ItemFn, Stmt};

use super::{helpers::*, ContractConditionsHandler, INTERNAL_RESULT_IDENT};

impl<'a> ContractConditionsHandler<'a> {
    /// Generate initial contract.
    ///
    /// 1. Generating the body for the recursion closure used by contracts.
    ///    - The recursion closure body will contain the check and replace closure.
    pub fn handle_untouched(&mut self) {
        let replace_name = &self.replace_name;
        let modifies_name = &self.modify_name;
        let recursion_name = &self.recursion_name;
        let check_name = &self.check_name;

        let replace_closure = self.replace_closure();
        let check_closure = self.check_closure();
        let recursion_closure = self.new_recursion_closure(&replace_closure, &check_closure);

        let span = Span::call_site();
        let replace_ident = Ident::new(&self.replace_name, span);
        let check_ident = Ident::new(&self.check_name, span);
        let recursion_ident = Ident::new(&self.recursion_name, span);

        // The order of `attrs` and `kanitool::{checked_with,
        // is_contract_generated}` is important here, because macros are
        // expanded outside in. This way other contract annotations in `attrs`
        // sees those attributes and can use them to determine
        // `function_state`.
        let ItemFn { attrs, vis, sig, block } = &self.annotated_fn;
        self.output.extend(quote!(
            #(#attrs)*
            #[kanitool::recursion_check = #recursion_name]
            #[kanitool::checked_with = #check_name]
            #[kanitool::replaced_with = #replace_name]
            #[kanitool::inner_check = #modifies_name]
            #vis #sig {
                // Dummy function used to force the compiler to capture the environment.
                // We cannot call closures inside constant functions.
                // This function gets replaced by `kani::internal::call_closure`.
                #[inline(never)]
                #[kanitool::fn_marker = "kani_register_contract"]
                pub const fn kani_register_contract<T, F: FnOnce() -> T>(f: F) -> T {
                    unreachable!()
                }
                let kani_contract_mode = kani::internal::mode();
                match kani_contract_mode {
                    kani::internal::RECURSION_CHECK => {
                        #recursion_closure;
                        kani_register_contract(#recursion_ident)
                    }
                    kani::internal::REPLACE => {
                        #replace_closure;
                        kani_register_contract(#replace_ident)
                    }
                    kani::internal::SIMPLE_CHECK => {
                        #check_closure;
                        kani_register_contract(#check_ident)
                    }
                    _ => #block
                }
            }
        ));
    }

    /// Handle subsequent contract attributes.
    ///
    /// Find the closures added by the initial setup, parse them and expand their body according
    /// to the attribute being handled.
    pub fn handle_expanded(&mut self) {
        let mut annotated_fn = self.annotated_fn.clone();
        let ItemFn { block, .. } = &mut annotated_fn;
        let recursion_closure = expect_closure_in_match(&mut block.stmts, "recursion_check");
        self.expand_recursion(recursion_closure);

        let replace_closure = expect_closure_in_match(&mut block.stmts, "replace");
        self.expand_replace(replace_closure);

        let check_closure = expect_closure_in_match(&mut block.stmts, "check");
        self.expand_check(check_closure);

        self.output.extend(quote!(#annotated_fn));
    }

    /// Generate the tokens for the recursion closure.
    fn new_recursion_closure(
        &self,
        replace_closure: &TokenStream,
        check_closure: &TokenStream,
    ) -> TokenStream {
        let ItemFn { ref sig, .. } = self.annotated_fn;
        let output = &sig.output;
        let span = Span::call_site();
        let result = Ident::new(INTERNAL_RESULT_IDENT, span);
        let replace_ident = Ident::new(&self.replace_name, span);
        let check_ident = Ident::new(&self.check_name, span);
        let recursion_ident = Ident::new(&self.recursion_name, span);

        quote!(
            #[kanitool::is_contract_generated(recursion_check)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #recursion_ident = || #output
            {
                #[kanitool::recursion_tracker]
                static mut REENTRY: bool = false;
                if unsafe { REENTRY } {
                    #replace_closure
                    #replace_ident()
                } else {
                    unsafe { REENTRY = true };
                    #check_closure
                    let #result = #check_ident();
                    unsafe { REENTRY = false };
                    #result
                }
            };
        )
    }

    /// Expand an existing recursion closure with the new condition.
    fn expand_recursion(&self, closure: &mut Stmt) {
        // TODO: Need to enter if / else. Make this traverse body and return list statements :(
        let body = closure_body(closure);
        let stmts = &mut body.block.stmts;
        let if_reentry = stmts
            .iter_mut()
            .find_map(|stmt| {
                if let Stmt::Expr(Expr::If(if_expr), ..) = stmt { Some(if_expr) } else { None }
            })
            .unwrap();

        let replace_closure = expect_closure(&mut if_reentry.then_branch.stmts, "replace");
        self.expand_replace(replace_closure);

        let else_branch = if_reentry.else_branch.as_mut().unwrap();
        let Expr::Block(else_block) = else_branch.1.as_mut() else { unreachable!() };
        let check_closure = expect_closure(&mut else_block.block.stmts, "check");
        self.expand_check(check_closure);
    }
}
