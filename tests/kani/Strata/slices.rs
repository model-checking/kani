// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Test slice support in Strata backend

#[kani::proof]
fn test_slice_from_array() {
    let arr: [u32; 5] = [1, 2, 3, 4, 5];
    let slice: &[u32] = &arr;
    assert!(slice[0] == 1);
    assert!(slice[4] == 5);
}

#[kani::proof]
fn test_slice_length() {
    let arr: [u32; 3] = [10, 20, 30];
    let slice: &[u32] = &arr;
    assert!(slice.len() == 3);
}

#[kani::proof]
fn test_slice_indexing() {
    let arr: [i32; 4] = [100, 200, 300, 400];
    let slice: &[i32] = &arr;
    let val = slice[2];
    assert!(val == 300);
}

#[kani::proof]
fn test_mutable_slice() {
    let mut arr: [u32; 3] = [1, 2, 3];
    let slice: &mut [u32] = &mut arr;
    slice[1] = 10;
    assert!(slice[1] == 10);
}

#[kani::proof]
fn test_slice_iteration() {
    let arr: [u32; 3] = [5, 10, 15];
    let slice: &[u32] = &arr;
    let mut sum: u32 = 0;
    let mut i: usize = 0;

    while i < slice.len() {
        sum = sum + slice[i];
        i = i + 1;
    }

    assert!(sum == 30);
}
