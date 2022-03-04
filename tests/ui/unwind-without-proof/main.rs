// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-unwinding-checks --function harness

// This test is to check Kani's error handling for harnesses that have unwind attributes
// without '#[kani::proof]' attribute declared

fn main() {
    assert!(1 == 2);
}

#[kani::unwind(7)]
pub fn harness() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 8);
    }
}
