// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let u64_1: u64 = kani::any();
    let u64_2: u64 = kani::any();
    let u64_3: u64 = kani::any();
    assert!(!(u64_1 == u64::MIN && u64_2 == 101 && u64_3 == u64::MAX));
}
