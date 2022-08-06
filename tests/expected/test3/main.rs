// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let mut a: f32 = 0.0;
    let mut i = 10;

    while i != 0 {
        a += 1.0;
        i -= 1;
    }

    // at this point, a == 10.0 and i == 0
    match kani::any::<i8>() {
        // should fail
        0 => assert!(a == 10.0 && i == 1),
        // should fail
        1 => assert!(a == 9.0 && i == 0),
        // should fail
        2 => assert!(a == 9.0 && i == 1),
        // should succeed
        3 => assert!(a == 10.0 && i == 0),
        // should succeed
        4 => assert!(a == 9.0 || i == 0),
        // should succeed
        5 => assert!(a == 10.0 || i == 1),
        // should fail
        6 => assert!(a == 9.0 || i == 1),
        // should succeed
        _ => assert!(a == 10.0 || i == 0),
    }
}
