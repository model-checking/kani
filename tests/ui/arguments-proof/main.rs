// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-unwinding-checks

// This test is to check Kani's error handling for harnesses that have proof attributes
// with arguments when the expected declaration takes no arguments.

fn main() {
    assert!(1 == 2);
}

#[kani::proof(some, argument2)]
fn harness() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}
