// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"
#![feature(proc_macro_diagnostic)]

mod derive;

// proc_macro::quote is nightly-only, so we'll cobble things together instead
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[cfg(kani_sysroot)]
use sysroot as attr_impl;

#[cfg(not(kani_sysroot))]
use regular as attr_impl;

/// Marks a Kani proof harness
///
/// For async harnesses, this will call [`block_on`](https://model-checking.github.io/kani/crates/doc/kani/futures/fn.block_on.html) to drive the future to completion (see its documentation for more information).
///
/// If you want to spawn tasks in an async harness, you have to pass a schedule to the `#[kani::proof]` attribute,
/// e.g. `#[kani::proof(schedule = kani::RoundRobin::default())]`.
///
/// This will wrap the async function in a call to [`block_on_with_spawn`](https://model-checking.github.io/kani/crates/doc/kani/futures/fn.block_on_with_spawn.html) (see its documentation for more information).
#[proc_macro_error]
#[proc_macro_attribute]
pub fn proof(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::proof(attr, item)
}

/// Specifies that a proof harness is expected to panic.**
///
/// This attribute allows users to exercise *negative verification*.
/// It's analogous to how
/// [`#[should_panic]`](https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html#testing-panics)
/// allows users to exercise [negative testing](https://en.wikipedia.org/wiki/Negative_testing)
/// for Rust unit tests.
///
/// # Limitations
///
/// The `#[kani::should_panic]` attribute verifies that there are one or more failed checks related to panics.
/// At the moment, it's not possible to pin it down to specific panics.
#[proc_macro_attribute]
pub fn should_panic(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::should_panic(attr, item)
}

/// Set Loop unwind limit for proof harnesses
/// The attribute `#[kani::unwind(arg)]` can only be called alongside `#[kani::proof]`.
/// arg - Takes in a integer value (u32) that represents the unwind value for the harness.
#[proc_macro_attribute]
pub fn unwind(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::unwind(attr, item)
}

/// Specify a function/method stub pair to use for proof harness
///
/// The attribute `#[kani::stub(original, replacement)]` can only be used alongside `#[kani::proof]`.
///
/// # Arguments
/// * `original` - The function or method to replace, specified as a path.
/// * `replacement` - The function or method to use as a replacement, specified as a path.
#[proc_macro_attribute]
pub fn stub(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::stub(attr, item)
}

/// Select the SAT solver to use with CBMC for this harness
///
/// The attribute `#[kani::solver(arg)]` can only be used alongside `#[kani::proof]`.
///
/// arg - name of solver, e.g. kissat
#[proc_macro_attribute]
pub fn solver(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::solver(attr, item)
}

/// Mark an API as unstable. This should only be used inside the Kani sysroot.
/// See https://model-checking.github.io/kani/rfc/rfcs/0006-unstable-api.html for more details.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn unstable(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::unstable(attr, item)
}

/// Allow users to auto generate Arbitrary implementations by using `#[derive(Arbitrary)]` macro.
#[proc_macro_error]
#[proc_macro_derive(Arbitrary)]
pub fn derive_arbitrary(item: TokenStream) -> TokenStream {
    derive::expand_derive_arbitrary(item)
}

///  Add a precondition to this function.
///
/// This is part of the function contract API, together with [`ensures`].
///
/// The contents of the attribute is a condition over the input values to the
/// annotated function. All Rust syntax is supported, even calling other
/// functions, but the computations must be side effect free, e.g. it cannot
/// perform I/O or use mutable memory.
///
/// Kani requires each function that uses a contract (this attribute or
/// [`ensures`]) to have at least one designated [`proof_for_contract`] harness
/// for checking the contract.
///
/// This attribute is part of the unstable contracts API and requires
/// `-Zfunction-contracts` flag to be used.
#[proc_macro_attribute]
pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::requires(attr, item)
}

/// Add a postcondition to this function.
///
/// This is part of the function contract API, together with [`requires`].
///
/// The contents of the attribute is a condition over the input values to the
/// annotated function *and* its return value, accessible as a variable called
/// `result`. All Rust syntax is supported, even calling other functions, but
/// the computations must be side effect free, e.g. it cannot perform I/O or use
/// mutable memory.
///
/// Kani requires each function that uses a contract (this attribute or
/// [`requires`]) to have at least one designated [`proof_for_contract`] harness
/// for checking the contract.
///
/// This attribute is part of the unstable contracts API and requires
/// `-Zfunction-contracts` flag to be used.
#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::ensures(attr, item)
}

/// Designates this function as a harness to check a function contract.
///
/// The argument to this macro is the relative path (e.g. `foo` or
/// `super::some_mod::foo` or `crate::SomeStruct::foo`) to the function, the
/// contract of which should be checked.
///
/// The harness is expected to set up the arguments that `foo` should be called
/// with and initialzied any `static mut` globals that are reachable. All of
/// these should be initialized to as general value as possible, usually
/// achieved using `kani::any`. The harness must call e.g. `foo` at least once
/// and if `foo` has type parameters, only one instantiation of those parameters
/// is admissible. Violating either results in a compile error.
///
/// If any of those types have special invariants you can use `kani::assume` to
/// enforce them, but other than condition on inputs and checks of outputs
/// should be in the contract, not the harness for maximum soundness.
///
/// This attribute is part of the unstable contracts API and requires
/// `-Zfunction-contracts` flag to be used.
#[proc_macro_attribute]
pub fn proof_for_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::proof_for_contract(attr, item)
}

/// `stub_verified(TARGET)` is a harness attribute (to be used on [`proof`] or
/// [`proof_for_contract`] function) that replaces all occurrences of `TARGET`
/// reachable from this harness with a stub generated from the contract on
/// `TARGET`.
/// 
/// To create a contract for `TARGET` you must decorate it with at least one
/// [`requires`] or [`ensures`] attribute.
/// 
/// You may use multiple `stub_verified` attributes on a single harness.
/// 
/// For more information see the [function contract
/// RFC](https://model-checking.github.io/kani/rfc/rfcs/0009-function-contracts.html).
/// 
/// This attribute is part of the unstable contracts API and requires
/// `-Zfunction-contracts` flag to be used.
#[proc_macro_attribute]
pub fn stub_verified(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::stub_verified(attr, item)
}

/// This module implements Kani attributes in a way that only Kani's compiler can understand.
/// This code should only be activated when pre-building Kani's sysroot.
#[cfg(kani_sysroot)]
mod sysroot {
    use proc_macro_error::{abort, abort_call_site};

    mod contracts;

    pub use contracts::{ensures, proof_for_contract, requires, stub_verified};

    use super::*;

    use {
        quote::{format_ident, quote},
        syn::parse::{Parse, ParseStream},
        syn::{parse_macro_input, ItemFn},
    };

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

    kani_attribute!(should_panic, no_args);
    kani_attribute!(solver);
    kani_attribute!(stub);
    kani_attribute!(unstable);
    kani_attribute!(unwind);
}

/// This module provides dummy implementations of Kani attributes which cannot be interpreted by
/// other tools such as MIRI and the regular rust compiler.
///
/// This allow users to use code marked with Kani attributes, for example, for IDE code inspection.
#[cfg(not(kani_sysroot))]
mod regular {
    use super::*;

    /// Encode a noop proc macro which ignores the given attribute.
    macro_rules! no_op {
        ($name:ident) => {
            pub fn $name(_attr: TokenStream, item: TokenStream) -> TokenStream {
                item
            }
        };
    }

    /// Add #[allow(dead_code)] to a proof harness to avoid dead code warnings.
    pub fn proof(_attr: TokenStream, item: TokenStream) -> TokenStream {
        let mut result = TokenStream::new();
        result.extend("#[allow(dead_code)]".parse::<TokenStream>().unwrap());
        result.extend(item);
        result
    }

    no_op!(should_panic);
    no_op!(solver);
    no_op!(stub);
    no_op!(unstable);
    no_op!(unwind);
    no_op!(requires);
    no_op!(ensures);
    no_op!(proof_for_contract);
    no_op!(stub_verified);
}
