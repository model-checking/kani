// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z unstable-options --cbmc-args --show-properties
//! Checks that CBMC's `--show-properties` output lists the harness properties

#[kani::proof]
fn main() {
    let x: u8 = kani::any();
    assert!(x.wrapping_add(0) == x);
}
