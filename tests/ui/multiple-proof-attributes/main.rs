// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-unwinding-checks --verbose

// This test is to check Kani's error handling for harnesses that have multiple proof annotations
// declared.

#[kani::proof]
#[kani::proof]
fn main() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}
