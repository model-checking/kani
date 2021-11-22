// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check failure for set len on drop case.

// rmc-verify-fail

fn main() {
    let mut v: Vec<u32> = Vec::new();
    v.extend(42..=42);
    assert!(v[0] == 41); // Incorrect value
}
