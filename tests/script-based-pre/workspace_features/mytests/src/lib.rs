// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A test crate with the use_mylib feature defined.

#[cfg(feature = "use_mylib")]
use mylib::add;

#[cfg(feature = "use_mylib")]
#[kani::proof]
fn check_mylib_add() {
    let result = add(2, 3);
    assert!(result == 5);
}

#[kani::proof]
fn check_basic() {
    assert!(1 + 1 == 2);
}
