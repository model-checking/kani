// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This example is a copy of the `cbmc` test in
// `src/test/kani/LoopLoop_NonReturning/main_no_unwind_asserts.rs`
//
// The verification output should show an unwinding assertion failure.
//
// In this test, we check that Kani warns the user about unwinding failures
// and makes a recommendation to fix the issue.
#[kani::proof]
#[kani::unwind(9)]
fn main() {
    let mut a: u32 = kani::any();

    if a < 1024 {
        loop {
            a = a / 2;

            if a == 0 {
                break;
            }
        }

        assert!(a == 0);
    }
}
