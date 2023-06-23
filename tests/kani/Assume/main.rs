// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let i: i32 = kani::any();
    kani::assume(i < 10);
    assert!(i < 20);
}

#[kani::proof]
fn verify_any_where() {
    // Only single digit values are legal
    let i: i32 = kani::any_where(|x| *x < 10);
    assert!(i < 20);
}
