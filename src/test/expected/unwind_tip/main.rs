// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// cbmc-flags: --unwind 9

include!("../../rmc-prelude.rs");

// This example is a copy of the `cbmc` test in
// `src/test/cbmc/LoopLoop_NonReturning/main_no_unwind_asserts.rs`
//
// The verification output should show an unwinding assertion failure.
//
// In this test, we check that RMC warns the user about unwinding failures
// and makes a recommendation to fix the issue.
fn main() {
    let mut a: u32 = __nondet();

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
