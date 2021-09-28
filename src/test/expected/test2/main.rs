// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
pub fn main() {
    let mut a: i32 = 4;
    let mut i = 0;
    while a != 1 {
        a >>= 1;
        i += 1;
    }

    // at this point, a == 1 and i == 2
    // should fail
    assert!(i == 3);
    // should succeed
    assert!(i == 2);
    // should succeed
    assert!(i == 2 || i == 3);
}
