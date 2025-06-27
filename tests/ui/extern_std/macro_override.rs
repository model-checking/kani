// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test if Kani can correctly identify assertions in a no_std crate that re-exports `std` library.
//! Issue previously reported here: <https://github.com/model-checking/kani/issues/2187>
//
// compile-flags: --cfg=feature="std"
#![no_std]

#[cfg(feature = "std")]
extern crate std;

#[kani::proof]
fn foo() {
    std::debug_assert!(true, "debug_assert");
    if kani::any_where(|b| !b) {
        std::unreachable!("unreachable")
    }
    if kani::any_where(|val: &u8| *val > 10) < 10 {
        std::panic!("panic")
    }
}
