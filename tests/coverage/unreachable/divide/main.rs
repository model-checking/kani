// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Test that checks for UNREACHABLE panics. The panic is reported as NONE for the assumption that the divisor is not zero.
fn divide(a: i32, b: i32) -> i32 {
    if b != 0 {
        return a / b;
    } else {
        panic!("Division by zero");
    }
}

#[kani::proof]
fn main() {
    let y: i32 = kani::any();
    kani::assume(y != 0);
    let result = divide(10, y);
    assert_eq!(result, 5);
}
