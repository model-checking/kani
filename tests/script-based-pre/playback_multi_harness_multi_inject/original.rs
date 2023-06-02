// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani correctly adds tests when multiple harnesses are
//! present with each having multiple unit tests injected.

extern crate kani;

#[cfg(kani)]
mod verify {
    use kani::cover;
    use std::convert::TryFrom;
    use std::num::NonZeroU8;

    #[kani::proof]
    fn try_nz_u8_replica1() {
        let val: u8 = kani::any();
        let result = NonZeroU8::try_from(val);
        match result {
            Ok(nz_val) => {
                kani::assume(val == 16); // stabilize value for playback
                cover!(true, "Ok"); // Cover 1
                assert_eq!(nz_val.get(), val);
            }
            Err(_) => {
                cover!(true, "Not ok"); // Cover 2
                assert_eq!(val, 0);
            }
        }
    }

    #[kani::proof]
    fn try_nz_u8_replica2() {
        let val: u8 = kani::any();
        let result = NonZeroU8::try_from(val);
        match result {
            Ok(nz_val) => {
                kani::assume(val == 16); // stabilize value for playback
                cover!(true, "Ok"); // Cover 1
                assert_eq!(nz_val.get(), val);
            }
            Err(_) => {
                cover!(true, "Not ok"); // Cover 2
                assert_eq!(val, 0);
            }
        }
    }
}
