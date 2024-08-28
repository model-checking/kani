// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn find_index(nums: &[i32], target: i32) -> Option<usize> {
    for (index, &num) in nums.iter().enumerate() {
        if num == target {
            return Some(index);
        }
    }
    None
}

#[kani::proof]
fn main() {
    let numbers = [10, 20, 30, 40, 50];
    let target = 30;
    let result = find_index(&numbers, target);
    assert_eq!(result, Some(2));
}
