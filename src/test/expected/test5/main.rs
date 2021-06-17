// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    // should succeed
    assert!(div(4, 2) == 2);
    // should fail
    assert!(div(6, 2) == 2);
}

fn div(a: i32, b: i32) -> i32 {
    a / b
}
