// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018

//! Test that we can properly handle panic messages with rust 2018.

include! {"../test.rs"}

#[kani::proof]
fn check_panic_2018() {
    check_panic();
}
