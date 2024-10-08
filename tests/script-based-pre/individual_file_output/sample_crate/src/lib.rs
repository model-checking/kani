// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that we can correctly generate tests from a cover statement and run them.

#[cfg(kani)]
mod verify {
    #[kani::proof]
    fn any_is_ok() {
        let result: Result<char, bool> = kani::any();
        kani::cover!(result.is_ok());
    }

    #[kani::proof]
    fn any_is_err() {
        let result: Result<char, bool> = kani::any();
        kani::cover!(result.is_err());
    }
}
