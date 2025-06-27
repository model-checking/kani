// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let i64_1: i64 = kani::any();
    let i64_2: i64 = kani::any();
    let i64_3: i64 = kani::any();
    let i64_4: i64 = kani::any();
    let i64_5: i64 = kani::any();
    assert!(
        !(i64_1 == i64::MIN && i64_2 == -101 && i64_3 == 0 && i64_4 == 101 && i64_5 == i64::MAX)
    );
}
