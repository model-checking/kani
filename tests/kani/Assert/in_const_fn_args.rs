// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that `assert!` with a custom message can be used in a const
//! fn

const fn const_add(x: i32, y: i32) {
    assert!(x + y == x, "some message");
}

#[kani::proof]
fn check() {
    let x = kani::any();
    let y = 0;
    const_add(x, y);
}
