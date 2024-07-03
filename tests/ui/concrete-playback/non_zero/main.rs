// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

use std::num::NonZeroU8;

#[kani::proof]
pub fn harness() {
    let non_zero_u8_1: NonZeroU8 = kani::any();
    let non_zero_u8_2: NonZeroU8 = kani::any();
    let non_zero_u8_3: NonZeroU8 = kani::any();
    unsafe {
        assert!(
            !(non_zero_u8_1 == NonZeroU8::new_unchecked(1)
                && non_zero_u8_2 == NonZeroU8::new_unchecked(101)
                && non_zero_u8_3 == NonZeroU8::new_unchecked(255))
        );
    }
}
