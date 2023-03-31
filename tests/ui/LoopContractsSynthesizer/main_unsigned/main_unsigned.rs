// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --synthesize-loop-contracts

// Check if goto-synthesizer is correctly called, and synthesizes the required
// loop invariants.

#[kani::proof]
fn main() {
    let mut x: u64 = kani::any_where(|i| *i > 1);

    while x > 1 {
        x = x - 1;
    }

    assert!(x == 1);
}
