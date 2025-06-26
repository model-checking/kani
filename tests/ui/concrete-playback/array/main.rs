// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
#[kani::unwind(10)]
pub fn harness() {
    let arr_1: [u8; 2] = kani::any();
    assert!(!(arr_1[0] == 101 && arr_1[1] == 102));
}
