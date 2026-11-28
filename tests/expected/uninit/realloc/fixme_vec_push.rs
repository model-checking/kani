// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Reading a `Vec` after it grows.
//! See https://github.com/model-checking/kani/issues/5165 for details.

#[kani::proof]
fn check_vec_with_capacity_push() {
    let n: usize = kani::any();
    kani::assume(n <= 2);
    let mut v: Vec<u8> = Vec::with_capacity(n);
    v.push(1);
    v.push(2);
    assert!(v[0] == 1 && v[1] == 2);
}
