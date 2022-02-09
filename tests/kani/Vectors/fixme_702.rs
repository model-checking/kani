// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Failing example from https://github.com/model-checking/kani/issues/702
fn main() {
    let mut v = Vec::new();
    v.push(72);
    v.push(2);
    v.push(3);
    v.push(4);
    v.push(5);
    assert!(v[0] == 72);
    assert!(v[1] == 2);
    assert!(v[2] == 3);
    assert!(v[3] == 4);
}
