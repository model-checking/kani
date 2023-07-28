// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn find_positive(nums: &[i32]) -> Option<i32> {
    for &num in nums {
        if num > 0 {
            return Some(num);
        }
    }
    // This part is unreachable if there is at least one positive number.
    None
}

#[kani::proof]
fn main() {
    let numbers = [-3, -1, 0, 2, 4];
    let result = find_positive(&numbers);
    assert_eq!(result, Some(2));
}
