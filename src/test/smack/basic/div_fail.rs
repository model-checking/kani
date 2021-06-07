// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @expect error

pub fn main() {
    let a = 2;
    let b = 3;
    assert!(b / a != 1);
}
