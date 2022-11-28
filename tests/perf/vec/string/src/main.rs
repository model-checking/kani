// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks the performance of pushing onto a vector of strings

const N: usize = 9;

#[kani::proof]
#[kani::unwind(10))]
fn main() {
    let mut v: Vec<String> = Vec::new();
    for _i in 0..N {
        v.push(String::from("ABC"));
    }
    assert_eq!(v.len(), N);
    let index: usize = kani::any();
    kani::assume(index < v.len());
    let x = &v[index];
    assert_eq!(*x, "ABC");
}
