// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let i8_1: i8 = kani::any();
    let i8_2: i8 = kani::any();
    let i8_3: i8 = kani::any();
    let i8_4: i8 = kani::any();
    let i8_5: i8 = kani::any();
    assert!(!(i8_1 == i8::MIN && i8_2 == -101 && i8_3 == 0 && i8_4 == 101 && i8_5 == i8::MAX));
}
