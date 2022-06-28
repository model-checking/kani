// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Append 2 arbitrary vectors, lengths at most 3 and 4 respectively,
// and assert they have been appended.
#[kani::proof]
fn main() {
    let mut v1: Vec<u128> = kani::any_vec::<3>();
    let mut v2: Vec<u128> = kani::any_vec::<4>();
    let v1_initial = v1.clone();
    let v2_initial = v2.clone();
    let l1 = v1.len();
    let l2 = v2.len();

    v1.append(v2);
    assert!(v2.len() == 0);
    assert!(v1.len() == (l1 + l2));

    assert!(v1[0..l1] == v1_initial);
    assert!(v1[l1..] == v2_initial);
}
