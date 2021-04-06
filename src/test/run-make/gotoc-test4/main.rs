// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut a = 4;
    let mut i = 0;
    while a != 1 {
        a = div(a, 2);
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

fn div(a: i32, b: i32) -> i32 {
    a / b
}
