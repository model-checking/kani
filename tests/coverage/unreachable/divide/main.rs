// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn divide(a: i32, b: i32) -> i32 {
    if b != 0 {
        return a / b;
    } else {
        // This part is unreachable since b != 0 was already checked.
        panic!("Division by zero");
    }
}

#[kani::proof]
fn main() {
    let result = divide(10, 2);
    assert_eq!(result, 5);
}
