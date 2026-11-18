// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Does not compile on purpose: a rejected argument must fail before compilation starts.

#[kani::proof]
fn trivial_harness() {
    let x: u32 = "not a u32";
    assert!(x == x);
}
