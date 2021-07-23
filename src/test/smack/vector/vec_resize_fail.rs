// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --no-memory-splitting --unroll=3
// @expect error
// rmc-verify-fail

pub fn main() {
    let mut v1: Vec<u64> = vec![0];
    let mut v2: Vec<u64> = vec![3];
    v1.append(&mut v2);
    assert!(v1[1] != 3);
}
