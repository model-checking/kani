// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let usize_1: usize = kani::any();
    let usize_2: usize = kani::any();
    let usize_3: usize = kani::any();
    assert!(!(usize_1 == usize::MIN && usize_2 == 101 && usize_3 == usize::MAX));
}
