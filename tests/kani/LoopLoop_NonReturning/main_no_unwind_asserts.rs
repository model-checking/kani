// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: --no-default-checks

// This example tests the Kani flag `--no-default-checks`
//
// It is the same as `main.rs` except for the flags passed to Kani and CBMC
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
