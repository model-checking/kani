// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut a: f32 = 0.0;
    let mut i = 10;

    while i != 0 {
        a += 1.0;
        i -= 1;
    }

    // at this point, a == 10.0 and i == 0
    // should fail
    assert!(a == 10.0 && i == 1);
    // should fail
    assert!(a == 9.0 && i == 0);
    // should fail
    assert!(a == 9.0 && i == 1);
    // should succeed
    assert!(a == 10.0 && i == 0);

    // should succeed
    assert!(a == 9.0 || i == 0);
    // should succeed
    assert!(a == 10.0 || i == 1);
    // should fail
    assert!(a == 9.0 || i == 1);
    // should succeed
    assert!(a == 10.0 || i == 0);
}
