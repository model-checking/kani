// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts -Z stubbing

// Issue #4591: the replacement admits i32::MAX, which overflows in caller.

#[kani::ensures(|result: &i32| *result >= 0)]
fn abs_val(x: i32) -> i32 {
    if x < 0 { -x } else { x }
}

fn stub_abs_val(_x: i32) -> i32 {
    let r: i32 = kani::any();
    kani::assume(r >= 0);
    r
}

fn caller(x: i32) -> i32 {
    abs_val(x) + 1
}

#[kani::proof]
#[kani::stub(abs_val, stub_abs_val)]
fn check_caller() {
    let x: i32 = kani::any();
    let result = caller(x);
    assert!(result >= 1);
}
