// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Special way we handle the first time we encounter a contract attribute on a
//! function.

use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{ExprClosure, FnArg, ItemFn, Signature};

use super::{helpers::*, ContractConditionsHandler, INTERNAL_RESULT_IDENT};

impl<'a> ContractConditionsHandler<'a> {
    /// Generate initial contract.
    ///
    /// 1. Generating the body for all the closures used by contracts.
    ///    - The recursion closure body is always the same, and it is generated in this stage.
    ///    - The other closures body will depend on which annotation is being processed.
    /// 2. Emitting the extended function with the new closures and the new contract attributes
    pub fn handle_untouched(&mut self) {
        let replace_name = &self.replace_name;
        let modifies_name = &self.modify_name;
        let recursion_name = &self.recursion_name;

        let recursion_closure = self.recursion_closure();
        let replace_closure = self.replace_closure();
        println!("{recursion_closure}");

        // The order of `attrs` and `kanitool::{checked_with,
        // is_contract_generated}` is important here, because macros are
        // expanded outside in. This way other contract annotations in `attrs`
        // sees those attributes and can use them to determine
        // `function_state`.
        let ItemFn { attrs, vis, sig, block } = &self.annotated_fn;
        self.output.extend(quote!(
            #(#attrs)*
            #[kanitool::checked_with = #recursion_name]
            #[kanitool::replaced_with = #replace_name]
            #[kanitool::inner_check = #modifies_name]
            #vis #sig {
                #replace_closure
                #recursion_closure
                // -- Now emit the original code.
                #block
            }
        ));
    }

    /// Generate the tokens for the recursion closure.
    fn recursion_closure(&self) -> TokenStream {
        let ItemFn { ref sig, .. } = self.annotated_fn;
        let (inputs, args) = closure_args(&sig.inputs);
        let output = &sig.output;
        let span = Span::call_site();
        let result = Ident::new(INTERNAL_RESULT_IDENT, span);
        let replace_ident = Ident::new(&self.replace_name, span);
        let check_ident = Ident::new(&self.check_name, span);
        let recursion_ident = Ident::new(&self.recursion_name, span);

        let replace_closure = self.replace_closure();
        let check_closure = self.check_closure();

        quote!(
            #[kanitool::is_contract_generated(recursion_check)]
            #[allow(dead_code, unused_variables, unused_mut)]
            let mut #recursion_ident = |#inputs| #output
            {
                static mut REENTRY: bool = false;
                if unsafe { REENTRY } {
                    #replace_closure
                    #replace_ident(#(#args),*)
                } else {
                    #check_closure
                    unsafe { REENTRY = true };
                    let #result = #check_ident(#(#args),*);
                    unsafe { REENTRY = false };
                    #result
                }
            };
        )
    }
}
