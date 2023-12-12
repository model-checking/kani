// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn check_number(num: i32) -> &'static str {
    if num > 0 {
        // The line is partially covered because the if statement is UNREACHABLE while the else statement is reachable
        if num % 2 == 0 { "Positive and Even" } else { "Positive and Odd" }
    } else if num < 0 {
        "Negative"
    } else {
        "Zero"
    }
}

#[kani::proof]
fn main() {
    let number = 7;
    let result = check_number(number);
    assert_eq!(result, "Positive and Odd");
}
