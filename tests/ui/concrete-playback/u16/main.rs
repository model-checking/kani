// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let u16_1: u16 = kani::any();
    let u16_2: u16 = kani::any();
    let u16_3: u16 = kani::any();
    assert!(!(u16_1 == u16::MIN && u16_2 == 101 && u16_3 == u16::MAX));
}
