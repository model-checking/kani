// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    let a: i32 = __nondet();
    if -100000 < a && a < 100000 {
        let b = -a;

        if a > 0 {
            assert!(a > b);
        } else if a < 0 {
            assert!(a < b - 1);
        } else {
            assert!(b == a);
        }
    }
}
