// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This harness should fail because unwind limit is too high.

fn main() {
    assert!(1 == 2);
}

#[kani::proof]
#[kani::unwind(9)]
fn harness() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}

#[kani::proof]
fn harness_2() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}
