// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that Kani reports the correct coverage results in the case of an
//! arithmetic overflow failure (caused by the second call to `cond_reduce`).

fn cond_reduce(thresh: u32, x: u32) -> u32 {
    if x > thresh { x - 50 } else { x } // PARTIAL: some cases are `COVERED`, others are not
}

#[kani::proof]
fn main() {
    cond_reduce(60, 70);
    cond_reduce(40, 42);
} // NONE: Caused by the arithmetic overflow failure from the second call to `cond_reduce`
