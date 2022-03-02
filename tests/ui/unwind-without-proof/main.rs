// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-unwinding-checks

// This test is to check Kani's error handling for harnesses that have unwind attributes
// without '#[kani::proof]' attribute declared

fn main() {
    assert!(1 == 2);
}

#[kani::unwind(9)]
fn harness_7() {
    let mut counter = 0;
    for i in 0..10 {
        counter += 1;
        assert!(counter < 8);
    }
}
