// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
fn main() {
    let mut x: Vec<i32> = Vec::new();
    x.push(10);
    assert!(x[0] == 10);
    let y = x.pop().unwrap();
    assert!(y == 10);
    assert!(y != 10);
}
