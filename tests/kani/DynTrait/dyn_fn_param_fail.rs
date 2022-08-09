// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn function pointer to a stand alone
// function definition. Inverted negative test, expected to fail
// all asserts.

#![feature(ptr_metadata)]

include!("../Helpers/vtable_utils_ignore.rs");
fn takes_dyn_fun(fun: &dyn Fn() -> u32) {
    let x = fun();
    if kani::any() {
        kani::expect_fail(x != 5, "Wrong return");
    }

    /* The function dynamic object has no associated data */
    kani::expect_fail(size_from_vtable(vtable!(fun)) != 0, "Wrong size");
}

pub fn unit_to_u32() -> u32 {
    if kani::any() {
        assert!(false);
    }
    5 as u32
}

#[kani::proof]
fn main() {
    takes_dyn_fun(&unit_to_u32)
}
