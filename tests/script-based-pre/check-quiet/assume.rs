// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn assume1() {
    let i: i32 = kani::any();
    kani::assume(i < 10);
    assert!(i < 20);
}

#[kani::proof]
fn assume2() {
    let i: u32 = kani::any();
    kani::assume(i < 10);
    assert!(i < 20);
}
