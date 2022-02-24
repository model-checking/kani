// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --output-format old
fn main() {
    let mut f: f32 = 2.0;
    while f != 0.0 {
        f -= 1.0;
    }

    match kani::nondet::<u8>() {
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
