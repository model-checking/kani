// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Append 2 arbitrary vectors, lengths at most 1 and 2 respectively,
/// and assert they have been appended.  This test case takes 53
/// minutes with #[kani::unwind(75)], but takes less than a second
/// with semantically equivalent exact_vec. This performance problem
/// is the subject of issue #1322.
#[kani::proof]
#[kani::unwind(1)]
fn main() {
    let mut v1: Vec<u128> = kani::vec::any_vec::<_, 1>();
    kani::assume(v1.len() == 1);
    let mut v2: Vec<u128> = kani::vec::any_vec::<_, 2>();
    kani::assume(v2.len() == 2);

    let v1_initial = v1.clone();
    let v2_initial = v2.clone();
    let l1 = v1.len();
    let l2 = v2.len();

    v1.append(&mut v2);
    assert!(v2.len() == 0);
    assert!(v1.len() == (l1 + l2));

    assert!(v1[0..l1] == v1_initial);
    assert!(v1[l1..] == v2_initial);
}
