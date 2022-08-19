// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --enable-unstable --concrete-playback=Print

#[kani::proof]
pub fn harness1() {
    let u8_1: u8 = kani::any();
    assert!(u8_1 != u8::MIN);
}

#[kani::proof]
pub fn harness2() {
    let u8_2: u8 = kani::any();
    assert!(u8_2 != u8::MAX);
}
