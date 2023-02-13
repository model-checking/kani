// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that concatenating two nondet arrays into a vector
//! preserves the values

#[kani::proof]
fn check_concat() {
    let arr1: [i32; 2] = kani::any();
    let arr2: [i32; 3] = kani::any();
    let mut v = Vec::new();
    v.extend_from_slice(&arr1);
    v.extend_from_slice(&arr2);
    assert_eq!(v[0], arr1[0]);
    assert_eq!(v[1], arr1[1]);
    assert_eq!(v[2], arr2[0]);
    assert_eq!(v[3], arr2[1]);
    assert_eq!(v[4], arr2[2]);
}

fn main() {}
