// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This test checks if we can specify multiple harnesses in the Cargo.toml file.

#[kani::proof]
pub fn foo() {
    assert_eq!(1 + 2, 3);
}

#[kani::proof]
pub fn bar() {
    assert_ne!(2, 3);
}
