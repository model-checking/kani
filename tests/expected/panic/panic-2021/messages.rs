// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2021

//! Test that we can properly handle panic messages with rust 2021.

include! {"../test.rs"}

#[kani::proof]
fn check_panic_2021() {
    check_panic();
}

#[kani::proof]
fn check_user_panic_macro() {
    panic_oob!("try_insert", 5, 3);
}
