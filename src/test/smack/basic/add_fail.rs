// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect error
// rmc-verify-fail

pub fn main() {
    let a = 2;
    let b = 3;
    assert!(a + b == 6);
}
