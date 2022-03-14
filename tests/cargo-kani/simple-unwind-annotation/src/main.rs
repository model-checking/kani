// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --function harness

// The expected file presently looks for "1 == 2" above.
// But eventually this test may start to fail as we might stop regarding 'main'
// as a valid proof harness, since it isn't annotated as such.
// This test should be updated if we go that route.

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
