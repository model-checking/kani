// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --synthesize-loop-contracts

// Check if goto-synthesizer is correctly called, and synthesizes the required
// loop invariants.

#[kani::proof]
fn main() {
    let mut y: i32 = kani::any_where(|i| *i > 0);

    while y > 0 {
        y = y - 1;
    }

    assert!(y == 0);
}
