// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    let a = return_u32();
    assert!(a < 10);
    assert!(return_u32() < 10);
    assert!(return_u64() >= 100);
    assert!(return_bool());
    assert!(return_f32() < 21.0);
    assert!(return_f64() < 11.0 && return_f64() > -11.0);
}
fn return_u32() -> u32 {
    let x: u32 = __nondet();

    if x < 10 {
        return x;
    } else {
        return 5;
    }
}
fn return_u64() -> u64 {
    let x: u64 = __nondet();

    if x > 100 {
        return x;
    } else {
        return 100;
    }
}
fn return_bool() -> bool {
    let x: bool = __nondet();
    if x {
        return x;
    } else {
        return !x;
    }
}
fn return_f32() -> f32 {
    let x = 10.0;
    let y: bool = __nondet();
    if y {
        return x / 2.0;
    } else {
        return x * 2.0;
    }
}
fn return_f64() -> f64 {
    let x: f64 = __nondet();
    if x <= 10.0 && x >= -10.0 {
        return x;
    } else {
        return 0.0;
    }
}
