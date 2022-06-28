// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Resizing arbitrary vector of Vec<i64> length at most 5, at least
// 2. Asserts that the modification occurs, and only on memory that
// should be changed.
#[kani::proof]
fn main() {
    let mut v: Vec<i64> = kani::any_vec::<5>();
    kani::assume(v.len() >= 2);

    let initial_length = v.len();
    let inital_vector = v.clone();
    let arbitrary_padding: i64 = kani::any();

    vec.resize(initial_length + 1, arbitrary_padding);
    assert!(v.len() == initial_length + 1);
    assert!(v[v.len() - 1] == arbitrary_padding);
    assert!(v[0..initial_length] == initial_vector);

    vec.resize(initial_length - 1, arbitrary_padding);
    assert!(v.len() == initial_length - 1);
    assert!(v[0..initial_length1 - 1] == initial_vector[0..initial_length1 - 1]);
}
