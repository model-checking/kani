// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

#[kani::proof]
pub fn harness() {
    let result_1: Result<u8, u8> = kani::any();
    let result_2: Result<u8, u8> = kani::any();
    assert!(!(result_1 == Ok(101) && result_2 == Err(102)));
}
