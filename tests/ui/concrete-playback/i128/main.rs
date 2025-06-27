// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let i128_1: i128 = kani::any();
    let i128_2: i128 = kani::any();
    let i128_3: i128 = kani::any();
    let i128_4: i128 = kani::any();
    let i128_5: i128 = kani::any();
    assert!(
        !(i128_1 == i128::MIN
            && i128_2 == -101
            && i128_3 == 0
            && i128_4 == 101
            && i128_5 == i128::MAX)
    );
}
