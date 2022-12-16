// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This tests that while resolving function names during stubbing we correctly
//! differentiate between local functions (specified with a relative path) and
//! functions from external crates (specified with initial qualifier `::`) when
//! both a module and the external crate have the same name.

use other_crate;

mod my_mod {
    fn zero() -> u32 {
        0
    }

    fn one() -> u32 {
        1
    }

    mod other_crate {
        pub fn magic_number() -> u32 {
            13
        }
    }

    #[kani::proof]
    // This stub should resolve to a local function
    #[kani::stub(zero, other_crate::magic_number)]
    // This stub should resolve to an external function
    #[kani::stub(one, ::other_crate::magic_number)]
    fn harness() {
        assert_eq!(zero(), 13);
        assert_eq!(one(), 42);
    }
}
