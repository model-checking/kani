// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn trivial_harness() {
    let x: u32 = kani::any();
    assert!(x == x);
}
