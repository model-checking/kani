// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-unwinding-checks

// This test is to check Kani's error handling for harnesses that have unwind attributes
// that have multiple arguments provided when only one is allowed.

fn main() {
    assert!(1 == 2);
}

#[kani::proof]
#[kani::unwind(10,5)]
fn harness_3() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}
