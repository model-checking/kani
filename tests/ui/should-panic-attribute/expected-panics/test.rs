// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: -Copt-level=1
//! Checks that verfication passes when `#[kani::should_panic]` is used and all
//! failures encountered are panics.

#[kani::proof]
#[kani::should_panic]
fn check() {
    if kani::any() {
        panic!("panicked on the `if` branch!");
    } else {
        panic!("panicked on the `else` branch!");
    }
}
