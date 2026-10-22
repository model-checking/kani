// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `cargo kani -Zlean` translates a dependency's generic function with its moves.

#[kani::proof]
fn main() {
    let s: dep::Slot<u32> = dep::Slot { value: None, count: 0 };
    let _ = dep::put(s, 7);
}
