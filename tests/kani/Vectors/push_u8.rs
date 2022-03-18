// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that Kani doesn't get stuck on a program involving a Vector of u8
// https://github.com/model-checking/kani/issues/703

#[kani::proof]
fn main() {
    let mut v: Vec<u8> = Vec::new();
    v.push(5);
    v.push(255);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], 5);
    assert_eq!(v[1], 255);
}
