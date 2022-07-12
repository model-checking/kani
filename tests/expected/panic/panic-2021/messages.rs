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
