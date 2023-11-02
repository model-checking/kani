// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Harness used to test that playback do not override assertion and panic functions.

#[kani::proof]
pub fn harness() {
    let v1: u32 = kani::any();
    let v2: u32 = kani::any();
    // avoid direct assignments to v1 to block constant propagation.
    kani::assume(v1 == v2);

    match v2 {
        0 => assert_eq!(v1, 1),
        1 => assert_eq!(v1, 0, "Found {v1} != 0"),
        2 => panic!("Found value {v1}"),
        _ => unreachable!("oops"),
    }
}
