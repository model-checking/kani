// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Regression test for https://github.com/model-checking/kani/issues/1489
// Tests that memcmp and memcpy can be called with dangling pointers if the count is zero.

#[kani::proof]
fn main() {
    // Vec::new() creates a dangling pointer
    assert_eq!(Vec::<u8>::new(), Vec::<u8>::new());
}
