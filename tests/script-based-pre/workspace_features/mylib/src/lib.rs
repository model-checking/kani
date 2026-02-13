// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A simple library without any features defined.

pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[kani::proof]
fn check_add() {
    let result = add(1, 2);
    assert!(result == 3);
}
