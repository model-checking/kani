// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --gen-exe-trace

#[kani::proof]
pub fn harness() {
    let u32_1: u32 = kani::any();
    let u32_2: u32 = kani::any();
    let u32_3: u32 = kani::any();
    assert!(!(u32_1 == u32::MIN && u32_2 == 101 && u32_3 == u32::MAX));
}
