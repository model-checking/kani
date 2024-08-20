// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that Kani reports the correct coverage results in the case of an
//! arithmetic overflow failure (caused by the second call to `cond_reduce`).

fn cond_reduce(thresh: u32, x: u32) -> u32 {
    if x > thresh { x - 50 } else { x }
}

#[kani::proof]
fn main() {
    cond_reduce(60, 70);
    cond_reduce(40, 42);
}
