// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-overflow-checks

// We use `--no-overflow-checks` in this test to avoid getting
// a verification failure:
// [main.NaN.1] line 25 NaN on * in var_30 * 0.0f: FAILURE
// Tracking issue: https://github.com/model-checking/rmc/issues/307

include!("../../rmc-prelude.rs");

fn main() {
    let mut x = 1;
    add_two(&mut x);
    assert!(x == 3);

    let mut y: i32 = 1;
    subtract_two(&mut y);
    assert!(y == -1);

    let mut z = false;
    make_true(&mut z);
    assert!(z);

    let mut a: f32 = __nondet();
    let b = a;
    div_by_two(&mut a);
    //       NaN
    assert!(a * 0.0 != 0.0 || a == b / 2.0);
}

fn add_two(a: *mut u32) {
    unsafe {
        *a += 2;
    }
}
fn subtract_two(a: *mut i32) {
    unsafe {
        *a -= 2;
    }
}
fn make_true(a: *mut bool) {
    unsafe {
        *a = true;
    }
}
fn div_by_two(a: *mut f32) {
    unsafe {
        *a /= 2.0;
    }
}
