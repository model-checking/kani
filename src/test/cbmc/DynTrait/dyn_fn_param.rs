// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn function pointer to a stand alone
// function definition

#![feature(raw)]
#![allow(deprecated)]

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
