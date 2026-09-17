// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Reproduces the libc (>= 0.2.188) pattern: a macro-generated internal prelude that
// re-exports `core::assert`, glob-imported by modules that call `assert!`. With Kani's
// macro-override injection this is ambiguous (E0659, GlobVsOuter, a hard error);
// `--no-assert-overrides` skips the injection so the crate compiles, at the cost of
// assertion failures being reported as generic panics.

macro_rules! make_prelude {
    () => {
        mod prelude {
            #[allow(unused_imports)]
            pub(crate) use core::assert;
        }
    };
}

make_prelude!();

mod uses_assert {
    use crate::prelude::*;

    pub fn checked_add(x: u8, y: u8) -> u8 {
        let sum = x.wrapping_add(y);
        assert!(sum >= x || sum < x); // trivially true; exercises the ambiguous macro
        sum
    }
}

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn check_add() {
        let x: u8 = kani::any();
        let y: u8 = kani::any();
        let _ = crate::uses_assert::checked_add(x, y);
    }

    // Assertion failures still fail verification without the overrides,
    // just with a generic panic message.
    #[kani::proof]
    fn check_assert_still_fails() {
        let x: u8 = kani::any();
        assert!(x < 255);
    }
}
