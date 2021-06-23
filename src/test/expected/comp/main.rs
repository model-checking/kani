// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[allow(dead_code)]
fn eq1(a: i32, b: i32) {
    assert!(a + b == b + a);
    assert!(a + b != a + b + 1);
}

#[allow(dead_code)]
fn eq2(a: i32, b: i32) {
    assert!(a + b > a);
    assert!(a - b < a);
}

include!("../../rmc-prelude.rs");

fn main() {
    let a = __nondet();
    let b = __nondet();
    if a > -400 && a < 100 && b < 200 && b > 0 {
        eq1(a, b);
        eq2(a, b);
    }
}
