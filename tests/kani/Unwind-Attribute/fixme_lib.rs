// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn main() {
    assert!(1 == 2);
}

// rmc-flags: --no-unwinding-checks

// Fix me
#[kani::proof]
#[kani::unwind(10)]
fn harness() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}

#[kani::unwind(8)]
fn harness_2() {
    let mut counter = 0;
    for i in 0..7 {
        counter += 1;
        assert!(counter < 5);
    }
}

#[kani::unwind(9)]
fn harness_3() {
    let mut counter = 0;
    for i in 0..10 {
        counter += 1;
        assert!(counter < 8);
    }
}
