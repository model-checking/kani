// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks writes to a local whose type is a raw pointer to a function item. Kani reads such a local
//! as a reference to the function item, so a write to it must not use that reference as its target.

fn plus_one(x: i32) -> i32 {
    x + 1
}

fn as_raw<F>(f: &F) -> *const F {
    f
}

/// The pointer is written by an assignment.
#[kani::proof]
fn check_assign() {
    let f = plus_one;
    let p: *const _ = &f;
    assert_eq!(unsafe { (*p)(1) }, 2);
}

/// The pointer is written by a call's return value.
#[kani::proof]
fn check_call_destination() {
    let f = plus_one;
    let p = as_raw(&f);
    assert_eq!(unsafe { (*p)(2) }, 3);
}
