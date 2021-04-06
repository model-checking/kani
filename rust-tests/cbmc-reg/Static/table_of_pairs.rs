// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
static TABLE: [(u64, u64); 2] = [(1, 2), (3, 4)];

// This is to ensure that we don't just constant propegate away the assertion
fn test_equal(a: u64, b: u64) -> bool {
    a == b
}

fn main() {
    let x = TABLE[0];
    assert!(test_equal(x.1, 2));
}
