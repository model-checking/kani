// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn check_number(num: i32) -> &'static str {
    if num > 0 {
        // The next line is partially covered
        if num % 2 == 0 { "Positive and Even" } else { "Positive and Odd" }
    } else if num < 0 { // From here on, only the terminator is covered
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
