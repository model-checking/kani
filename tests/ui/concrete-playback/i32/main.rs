// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let i32_1: i32 = kani::any();
    let i32_2: i32 = kani::any();
    let i32_3: i32 = kani::any();
    let i32_4: i32 = kani::any();
    let i32_5: i32 = kani::any();
    assert!(
        !(i32_1 == i32::MIN && i32_2 == -101 && i32_3 == 0 && i32_4 == 101 && i32_5 == i32::MAX)
    );
}
