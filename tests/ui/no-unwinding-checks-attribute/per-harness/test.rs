// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Checks that `#[kani::no_unwinding_checks]` turns off the unwinding assertions of the harness
//! it annotates, and of no other harness. Both harnesses run the same loop under the same bound,
//! which inputs of 3 or more exceed.

#[kani::proof]
#[kani::unwind(3)]
#[kani::no_unwinding_checks]
fn check_without_unwinding_checks() {
    let n: u8 = kani::any();
    let mut count = 0;
    while count < n {
        count += 1;
    }
    assert!(count == n);
}

#[kani::proof]
#[kani::unwind(3)]
fn check_with_unwinding_checks() {
    let n: u8 = kani::any();
    let mut count = 0;
    while count < n {
        count += 1;
    }
    assert!(count == n);
}
