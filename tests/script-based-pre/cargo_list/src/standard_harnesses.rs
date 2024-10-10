// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Test that the cargo list command can find Kani attributes across multiple files.

#[cfg(kani)]
mod example {
    mod verify {
        #[kani::proof]
        fn check_modify() {}

        #[kani::proof]
        fn check_new() {}
    }
}
