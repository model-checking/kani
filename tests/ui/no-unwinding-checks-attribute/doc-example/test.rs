// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The example of `#[kani::no_unwinding_checks]` in docs/src/reference/attributes.md: the harness verifies,
//! and the result covers the values of `n` below 3 only.

#[kani::proof]
#[kani::unwind(3)]
#[kani::no_unwinding_checks]
fn check_count_up_to_n() {
    let n: u8 = kani::any();
    let mut count = 0;
    while count < n {
        count += 1;
    }
    assert!(count == n);
    kani::cover!(n == 2, "`n` = 2 is reached");
    kani::cover!(n == 3, "`n` = 3 is reached");
}
