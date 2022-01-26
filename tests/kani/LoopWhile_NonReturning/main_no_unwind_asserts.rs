// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-default-checks
// cbmc-flags: --unwind 10

// This example tests the Kani flag `--no-default-checks`
//
// It is the same as `main.rs` except for the flags passed to Kani and CBMC
// Running it with `--unwind 10` is not enough to unwind the loop
// and generates this verification output:
//
// [main.unwind.0] line 30 unwinding assertion loop 0: FAILURE
// [main.assertion.1] line 34 assertion failed: a == 0: SUCCESS
//
// ** 1 of 2 failed (2 iterations)
// VERIFICATION FAILED
//
// But with `--no-default-checks` we will avoid introducing a unwinding assertion:
//
// [main.assertion.1] line 34 assertion failed: a == 0: SUCCESS
//
// ** 0 of 1 failed (1 iterations)
// VERIFICATION SUCCESSFUL
fn main() {
    let mut a: u32 = kani::any();

    if a < 1024 {
        while a > 0 {
            a = a / 2;
        }

        assert!(a == 0);
    }
}
