// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// compile-flags: --edition 2018
// Check that an assert that uses a pre-Rust-2018 style, where a string is
// directly used without a format string literal is accepted by Kani
// This was previously failing:
// https://github.com/model-checking/kani/issues/3717

#[kani::proof]
fn check_assert_2018() {
    let s = String::new();
    // This is deprecated in Rust 2018 and is a hard error starting Rust 2021.
    // One should instead use:
    // ```
    // assert!(true, "{}", s);
    // ```
    assert!(true, s);
}
