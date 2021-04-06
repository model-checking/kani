// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn add2(a: i32, b: i32) -> f32 {
    add(a, b as f32)
}

fn main() {
    // should succeed: 1 + 1 = 2
    assert!(add2(1, 1) == 2.0);
    // should fail: 2 + 1 = 3
    assert!(add2(2, 1) == 2.0);
}

fn add(a: i32, b: f32) -> f32 {
    a as f32 + b
}
