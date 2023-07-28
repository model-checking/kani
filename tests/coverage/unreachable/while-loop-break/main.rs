// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks coverage results in an example with a `while` loop that returns before
//! running the last iteration.

fn find_first_negative(nums: &[i32]) -> Option<i32> {
    let mut index = 0;
    while index < nums.len() {
        if nums[index] < 0 {
            return Some(nums[index]);
        }
        index += 1;
    }
    None
}

#[kani::proof]
fn main() {
    let numbers = [1, 2, -3, 4, -5];
    let result = find_first_negative(&numbers);
    assert_eq!(result, Some(-3));
}
