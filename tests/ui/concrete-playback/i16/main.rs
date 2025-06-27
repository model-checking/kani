// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let i16_1: i16 = kani::any();
    let i16_2: i16 = kani::any();
    let i16_3: i16 = kani::any();
    let i16_4: i16 = kani::any();
    let i16_5: i16 = kani::any();
    assert!(
        !(i16_1 == i16::MIN && i16_2 == -101 && i16_3 == 0 && i16_4 == 101 && i16_5 == i16::MAX)
    );
}
