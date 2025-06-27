// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let u128_1: u128 = kani::any();
    let u128_2: u128 = kani::any();
    let u128_3: u128 = kani::any();
    assert!(!(u128_1 == u128::MIN && u128_2 == 101 && u128_3 == u128::MAX));
}
