// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --gen-exe-trace

#[kani::proof]
pub fn harness() {
    let u8_1: u8 = kani::any();
    let u8_2: u8 = kani::any();
    assert!(u8_1 != 101);
}
