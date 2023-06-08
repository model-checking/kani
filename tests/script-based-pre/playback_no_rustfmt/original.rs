// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani correctly adds tests to the cover checks reachable in a harness.
extern crate kani;

#[cfg(kani)]
mod verify {
    use kani::cover;
    use std::convert::TryFrom;
    use std::num::NonZeroU8;

    #[kani::proof]
    fn try_nz_u8() {
        let val: u8 = kani::any();
        let result = NonZeroU8::try_from(val);
        match result {
            Ok(nz_val) => {
                cover!(true, "Ok");
                assert_eq!(nz_val.get(), val);
            }
            Err(_) => {
                cover!(true, "Not ok");
                assert_eq!(val, 0);
            }
        }
    }
}
