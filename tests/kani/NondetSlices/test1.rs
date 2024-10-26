// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Test the Kani library's API for creating a non-det slice of a given array

fn check(slice: &[u8]) {
    let len = slice.len();
    assert!(len >= 0 && len <= 3, "Expected slice length to be between 0 and 3. Got {}.", len);
    if len > 0 {
        let elem = slice[0];
        assert!(
            elem == 1 || elem == 2 || elem == 3,
            "Expected a value of 1, 2, or 3 for the first element. Got {}.",
            elem
        );
    }
}

#[kani::proof]
fn main() {
    let arr = [1, 2, 3];
    // The slice returned can be any of the following:
    // {[], [1], [2], [3], [1, 2], [2, 3], [1, 2, 3]}
    let slice = kani::slice::any_slice_of_array(&arr);
    check(slice);
}
