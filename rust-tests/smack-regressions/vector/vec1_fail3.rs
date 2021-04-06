// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --no-memory-splitting
// @expect error

pub fn main() {
    let mut v: Vec<u64> = Vec::new();
    v.push(0);
    v.push(1);
    v.push(3);
    assert!(v[0] == 0);
    assert!(v[1] == 1);
    assert!(v[2] == 3);
    v[2] = v[0] + v[1];
    assert!(v[0] == 0);
    assert!(v[1] == 1);
    assert!(v[2] != 1);
}
