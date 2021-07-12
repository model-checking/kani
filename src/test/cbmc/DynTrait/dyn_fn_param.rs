// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//rmc-flags: --no-memory-safety-checks

// We use `--no-memory-safety-checks` in this test to avoid getting
// a verification failure:
// [pointer_dereference.14] invalid function pointer: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

// Check that we can pass a dyn function pointer to a stand alone
// function definition

#![feature(ptr_metadata)]

include!("../Helpers/vtable_utils_ignore.rs");

fn takes_dyn_fun(fun: &dyn Fn() -> u32) {
    let x = fun();
    assert!(x == 5);

    /* The function dynamic object has no associated data */
    assert!(size_from_vtable(vtable!(fun)) == 0);
}

pub fn unit_to_u32() -> u32 {
    5 as u32
}

fn main() {
    takes_dyn_fun(&unit_to_u32)
}
