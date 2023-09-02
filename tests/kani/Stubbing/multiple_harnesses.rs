// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//! Check that Kani can handle a different combination of stubs in
//! the same crate.

pub fn magic_number() -> u32 {
    0
}

pub fn magic_number_stub_1() -> u32 {
    1
}

pub fn magic_number_stub_2() -> u32 {
    2
}

#[cfg(kani)]
mod verify {
    use super::*;

    #[kani::proof]
    fn check_no_stub() {
        assert_eq!(magic_number(), 0);
    }

    #[kani::proof]
    #[kani::stub(magic_number, magic_number_stub_1)]
    fn check_stub_1() {
        assert_eq!(magic_number(), 1);
    }

    #[kani::proof]
    #[kani::stub(magic_number, magic_number_stub_2)]
    fn check_stub_2() {
        assert_eq!(magic_number(), 2);
    }
}
