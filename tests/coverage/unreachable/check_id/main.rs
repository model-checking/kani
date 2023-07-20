// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn foo(x: i32) {
    assert!(1 + 1 == 2);
    if x < 9 {
        // unreachable
        assert!(2 + 2 == 4);
    }
}

#[kani::proof]
fn main() {
    assert!(1 + 1 == 2);
    let x = if kani::any() { 33 } else { 57 };
    foo(x);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(3 + 3 == 5);
}
