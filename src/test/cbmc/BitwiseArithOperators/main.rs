// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    assert!(3 | 5 == 7);
    assert!(7 & 9 == 1);
    assert!(9 ^ 7 == 14);
    assert!(!8 ^ !0 == 8);

    let x = 1;
    let a: u32 = __nondet();
    let b: u32 = __nondet();
    if a < 100000 && b < 100000 {
        let c = a + b;
        if c & x == x {
            let d = a ^ b;
            assert!(d & x == x);
        } else {
            assert!(a & x == b & x)
        }
    }
}
