// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --harness harness --enable-unstable --concrete-playback=Print

#[kani::proof]
pub fn harness() {
    let u8_1: u8 = kani::any();
    let u8_2: u8 = kani::any();
    let u8_3: u8 = kani::any();
    assert!(!(u8_1 == u8::MIN && u8_2 == 101 && u8_3 == u8::MAX));
}
