// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn function pointer to a closure that captures
// some data
#![feature(ptr_metadata)]

include!("../Helpers/vtable_utils_ignore.rs");

fn takes_dyn_fun(fun: &dyn Fn() -> i32) {
    let x = fun();
    assert!(x == 5);
    /* The closure captures `a` and thus has nonzero size */
    assert!(size_from_vtable(vtable!(fun)) == 8);
}

fn main() {
    let a = vec![3];
    let closure = || a[0] + 2;
    takes_dyn_fun(&closure)
}
