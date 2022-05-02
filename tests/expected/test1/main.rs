// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let mut a: i32 = 0;
    let mut i: i32 = 10;
    while i != 0 {
        a += i;
        i -= 1;
    }
    // at this point, a == 55, i == 0
    // should fail
    assert!(a == 54);
    // should succeed
    assert!(a == 55);
    // should fail
    assert!(a >= 55);
}
