// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dummy test to check --mir-linker runs on a cargo project.
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[kani::proof]
fn check_add() {
    let result = add(2, 2);
    assert_eq!(result, 4);
}
