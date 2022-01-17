// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    assert!(1 == 2);
}

// rmc-flags: --no-unwinding-checks

// The expected file presently looks for "1 == 2" above.
// But eventually this test may start to fail as we might stop regarding 'main'
// as a valid proof harness, since it isn't annotated as such.
// This test should be updated if we go that route.

#[rmc::unwind(10)]
fn harness() {
    let mut counter = 0;
    while true {
        counter += 1;
        assert!(counter < 10);
    }
}

#[rmc::unwind(7)]
fn harness_2() {
    let mut counter = 0;
    for i in 0..7 {
        counter += 1;
        assert!(counter < 5);
    }
}
