// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Sort an arbitrary Vec<u32> of length 3. Assert that the sorting worked.
#[kani::proof]
#[kani::unwind(10)]
fn main() {
    // let mut v: Vec<u32> = kani::vec::any_vec::<_, 3>();
    let slice = kani::slice::any_slice::<u32, 3>();
    let mut v: Vec<u32> = slice.to_vec(); //kani::vec::any_vec::<_, 3>();
    kani::assume(v.len() == 3);
    v.sort();

    assert!(v[0] <= v[1]);
    assert!(v[0] <= v[2]);
}
