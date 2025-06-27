// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Zconcrete-playback --concrete-playback=print

//! Multiple harnesses with the same name but under different modules.

mod first {
    #[kani::proof]
    pub fn harness() {
        let u8_1: u8 = kani::any();
        assert!(u8_1 != u8::MIN);
    }
}

mod second {
    #[kani::proof]
    pub fn harness() {
        let u8_2: u8 = kani::any();
        assert!(u8_2 != u8::MAX);
    }
}
