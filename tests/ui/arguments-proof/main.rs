// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-unwinding-checks -Z async-lib

// This test is to check Kani's error handling for harnesses that have proof attributes
// with arguments when the expected declaration takes no arguments.

#[kani::proof]
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

// Test what happens if the schedule option is incorrect:

struct NotASchedule;

#[kani::proof(schedule = NotASchedule)]
async fn test() {
    assert!(true);
}
