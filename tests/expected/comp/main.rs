// Copyright Kani Contributors
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

#[kani::proof]
fn main() {
    let a = kani::any();
    let b = kani::any();
    if a > -400 && a < 100 && b < 200 && b > 0 {
        eq1(a, b);
        eq2(a, b);
    }
}
