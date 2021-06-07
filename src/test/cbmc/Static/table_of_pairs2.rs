// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
const SIZE1: usize = 1;
const SIZE2: usize = 2;

pub static TABLE1: [(u64, u64); SIZE1] = [(0, 1)];

pub static TABLE2: [(u64, u64); SIZE2] = [(2, 3), (4, 5)];

// This is to ensure that we don't just constant propagate away the assertion
fn test_equal(a: u64, b: u64) -> bool {
    a == b
}

fn main() {
    let x = TABLE1[0];
    assert!(test_equal(x.1, 1));
    let y = TABLE2[0];
    assert!(test_equal(y.1, 3));
    let z = TABLE2[1];
    assert!(test_equal(z.1, 5));
}
