// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Special way we handle the first time we encounter a contract attribute on a
//! function.

use proc_macro2::Span;
use quote::quote;
use syn::ItemFn;

use super::{
    helpers::*,
    shared::{attach_require_kani_any, identifier_for_generated_function},
    ContractConditionsData, ContractConditionsHandler,
};

impl<'a> ContractConditionsHandler<'a> {
    /// The complex case. We are the first time a contract is handled on this function, so
    /// we're responsible for
    ///
    /// 1. Generating a name for the check function
    /// 2. Emitting the original, unchanged item and register the check
    ///    function on it via attribute
    /// 3. Renaming our item to the new name
    /// 4. And (minor point) adding #[allow(dead_code)] and
    ///    #[allow(unused_variables)] to the check function attributes
    pub fn handle_untouched(&mut self) {
        // We'll be using this to postfix the generated names for the "check"
        // and "replace" functions.
        let item_hash = self.hash.unwrap();

        let original_function_name = self.annotated_fn.sig.ident.clone();

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
        let replace_fn_name_str = syn::LitStr::new(&replace_fn_name.to_string(), Span::call_site());
        let wrapper_fn_name_str =
            syn::LitStr::new(&self.make_wrapper_name().to_string(), Span::call_site());
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
        let is_impl_fn = is_probably_impl_fn(&self.annotated_fn);
        let ItemFn { attrs, vis, sig, block } = &self.annotated_fn;
        self.output.extend(quote!(
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

        self.output.extend(quote!(
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

        self.emit_check_function(Some(check_fn_name));
        self.emit_replace_function(Some(replace_fn_name));
        self.emit_augmented_modifies_wrapper();
    }
}
