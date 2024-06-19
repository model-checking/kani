// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Sort an arbitrary Vec<u32> of length 3. Assert that the sorting worked.
#[kani::proof]
#[kani::unwind(4)]
fn main() {
    let mut v: Vec<u32> = kani::vec::any_vec::<_, 2>();
    kani::assume(v.len() == 2);

    if v[0] > v[1] {
        v.reverse();
    }
    assert!(v[0] <= v[1]);
}
