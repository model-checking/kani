// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let mut f: f32 = 2.0;
    while f != 0.0 {
        f -= 1.0;
    }

    match kani::any::<u8>() {
        // at this point, f == 0.0
        // should succeed
        0 => assert!(1.0 / f != 0.0 / f),
        // should succeed
        1 => assert!(!(1.0 / f == 0.0 / f)),
        // should fail
        2 => assert!(1.0 / f == 0.0 / f),
        // should fail
        3 => assert!(0.0 / f == 0.0 / f),
        // should suceed
        _ => assert!(0.0 / f != 0.0 / f),
    }
}
