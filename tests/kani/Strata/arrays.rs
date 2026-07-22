// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test array support in Strata backend

#[kani::proof]
fn test_array_literal() {
    let arr: [u32; 3] = [1, 2, 3];
    assert!(arr[0] == 1);
    assert!(arr[1] == 2);
    assert!(arr[2] == 3);
}

#[kani::proof]
fn test_array_indexing() {
    let arr: [i32; 5] = [10, 20, 30, 40, 50];
    let x = arr[2];
    assert!(x == 30);
}

#[kani::proof]
fn test_array_assignment() {
    let mut arr: [u32; 3] = [0, 0, 0];
    arr[0] = 5;
    arr[1] = 10;
    arr[2] = 15;
    assert!(arr[0] == 5);
    assert!(arr[1] == 10);
    assert!(arr[2] == 15);
}

#[kani::proof]
fn test_array_length() {
    let arr: [u32; 10] = [0; 10];
    let len = arr.len();
    assert!(len == 10);
}

#[kani::proof]
fn test_array_iteration() {
    let arr: [u32; 3] = [1, 2, 3];
    let mut sum: u32 = 0;
    let mut i: usize = 0;

    while i < 3 {
        sum = sum + arr[i];
        i = i + 1;
    }

    assert!(sum == 6);
}

#[kani::proof]
fn test_array_bounds() {
    let arr: [u32; 5] = [1, 2, 3, 4, 5];
    let idx: usize = 2;
    let val = arr[idx];
    assert!(val == 3);
}
