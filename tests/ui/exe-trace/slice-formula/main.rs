// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --gen-exe-trace

#[kani::proof]
pub fn harness() {
    let _u8_1: u8 = kani::any();
    let u8_2: u16 = kani::any();
    let _u8_3: u32 = kani::any();
    let u8_4: u64 = kani::any();
    assert!(!(u8_2 == 101 && u8_4 == 102));
}
