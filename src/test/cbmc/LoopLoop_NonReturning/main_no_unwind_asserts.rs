// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// rmc-flags: --no-default-checks
// cbmc-flags: --unwind 9

include!("../../rmc-prelude.rs");

// This example tests the RMC flag `--no-default-checks`
//
// It is the same as `main.rs` except for the flags passed to RMC and CBMC
// Running it with `--unwind 10` is not enough to unwind the loop
// and generates this verification output:
//
// [main.unwind.0] line 30 unwinding assertion loop 0: FAILURE
// [main.assertion.1] line 38 assertion failed: a == 0: SUCCESS
// ** 1 of 2 failed (2 iterations)
// VERIFICATION FAILED
//
// But with `--no-default-checks` we will avoid introducing a unwinding assertion:
//
// [main.assertion.1] line 38 assertion failed: a == 0: SUCCESS
//
// ** 0 of 1 failed (1 iterations)
// VERIFICATION SUCCESSFUL
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
