// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let i: i32 = kani::any();
    kani::assume(i < 10);
    assert!(i < 20);
}

#[kani::proof]
fn verify_any_value_where() {
    let i: i32 = kani::any_value_where(|x| *x < 10, "Only single digit values are legal");
    assert!(i < 20);
}
