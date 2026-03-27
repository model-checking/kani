// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stubbing multiple foreign functions from the same extern block,
//! and that non-stubbed foreign functions are unaffected.
//! Regression test for https://github.com/model-checking/kani/issues/2686

extern "C" {
    fn foreign_add(a: u32, b: u32) -> u32;
    fn foreign_mul(a: u32, b: u32) -> u32;
}

fn stub_add(_a: u32, _b: u32) -> u32 {
    100
}

fn stub_mul(_a: u32, _b: u32) -> u32 {
    200
}

#[kani::proof]
#[kani::stub(foreign_add, stub_add)]
#[kani::stub(foreign_mul, stub_mul)]
fn check_multiple_foreign_stubs() {
    assert_eq!(unsafe { foreign_add(1, 2) }, 100);
    assert_eq!(unsafe { foreign_mul(3, 4) }, 200);
}
