// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

static X: i32 = 12;

fn foo() -> i32 {
    X
}

#[kani::proof]
fn main() {
    assert!(10 == foo());
    assert!(12 == foo());
}
