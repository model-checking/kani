// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the behavior of Kani's `--no-codegen` option when the crate
//! has compilation errors

#[kani::proof]
fn main() {
    let x: i32 = 5;
    // Error: different types
    let y: u32 = x;
    assert_eq!(y, 5);
}
