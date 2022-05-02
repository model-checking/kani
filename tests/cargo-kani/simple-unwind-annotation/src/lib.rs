// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// TODO: When unwind is hooked up, `harness.expected` should now see success
#[kani::proof]
#[kani::unwind(9)]
fn harness_1() {
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
