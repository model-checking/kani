// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --cbmc-timeout 10
//
// Check the behavior of Kani when given a timeout via `--cbmc-timeout`, but
// CBMC completes before the timeout

#[kani::proof]
fn check_cbmc_no_timeout() {
    let x: u8 = kani::any();
    let y: u8 = kani::any();
    kani::assume(y == 0);
    assert_eq!(x + y, x);
}
