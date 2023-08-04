// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// #![feature(register_tool)]
// #![register_tool(kanitool)]
// Frustratingly, it's not enough for our crate to enable these features, because we need all
// downstream crates to enable these features as well.
// So we have to enable this on the commandline (see kani-rustc) with:
//   RUSTFLAGS="-Zcrate-attr=feature(register_tool) -Zcrate-attr=register_tool(kanitool)"
#![feature(let_chains, proc_macro_diagnostic, box_patterns)]

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
/// For async harnesses, this will call [`block_on`] to drive the future to completion (see its documentation for more information).
///
/// If you want to spawn tasks in an async harness, you have to pass a schedule to the `#[kani::proof]` attribute,
/// e.g. `#[kani::proof(schedule = kani::RoundRobin::default())]`.
/// This will wrap the async function in a call to [`block_on_with_spawn`] (see its documentation for more information).
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
/// The attribute '#[kani::unwind(arg)]' can only be called alongside '#[kani::proof]'.
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
/// The attribute `#[kani::solver(arg)]` can only be used alongside `#[kani::proof]``
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

/// Add a precondition to this function.
///
/// This is part of the function contract API, together with [`ensures`].
///
/// The contents of the attribute is a condition over the input values to the
/// annotated function. All Rust syntax is supported, even calling other
/// functions, but the computations must be side effect free, e.g. it cannot
/// perform I/O or use mutable memory.
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
#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::ensures(attr, item)
}

#[proc_macro_attribute]
pub fn proof_for_contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    attr_impl::proof_for_contract(attr, item)
}

/// This module implements Kani attributes in a way that only Kani's compiler can understand.
/// This code should only be activated when pre-building Kani's sysroot.
//#[cfg(kani_sysroot)]
mod sysroot;

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
}
