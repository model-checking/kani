// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that we can pass a dyn function pointer to a simple closure
#![feature(ptr_metadata)]

include!("../Helpers/vtable_utils_ignore.rs");
fn takes_dyn_fun(fun: &dyn Fn() -> i32) {
    let x = fun();
    kani::expect_fail(x != 5, "Wrong return");
    /* The closure does not capture anything and thus has zero size */
    kani::expect_fail(size_from_vtable(vtable!(fun)) != 0, "Wrong size");
}
fn main() {
    let closure = || 5;
    takes_dyn_fun(&closure)
}
