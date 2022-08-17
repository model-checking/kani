// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-unwinding-checks

// This test is to check Kani's error handling for harnesses that have unwind attributes
// that have multiple arguments provided when only one is allowed.

#[kani::proof]
fn main() {
    assert!(1 == 2);
}

#[kani::proof]
#[kani::unwind(10, 5)]
fn harness() {
    let mut counter = 0;
    loop {
        counter += 1;
        assert!(counter < 10);
    }
}
