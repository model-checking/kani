// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that Kani can verify crates that forbid(unstable_features) (e.g. rustls).
// Kani injects `feature(register_tool)` into every crate it compiles, which used to make
// such crates fail with "error: use of an unstable feature"; the driver now downgrades
// that lint with `--force-warn`.

#![forbid(unstable_features)]

fn add(x: u8, y: u8) -> u8 {
    x.wrapping_add(y)
}

#[kani::proof]
fn check_add() {
    let x: u8 = kani::any();
    let y: u8 = kani::any();
    assert!(add(x, y) == x.wrapping_add(y));
}
