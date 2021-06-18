// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
include!("../../rmc-prelude.rs");

fn main() {
    let a: u32 = __nondet();

    if a % 3 == 0 {
        assert!(a != 4);
    }

    if a % 3 == 0 {
        assert!(a != 5);
    } else if a % 3 == 1 {
        assert!(a > 0);
    } else {
        assert!(a > 1);
    }
}
