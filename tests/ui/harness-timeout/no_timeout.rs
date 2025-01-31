// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --harness-timeout 10 -Zunstable-options
//
// This test covers the case where a timeout is specified via `--harness-timeout`, but
// CBMC completes before the timeout is reached

#[kani::proof]
fn check_harness_no_timeout() {
    let x: u8 = kani::any();
    let y: u8 = kani::any();
    kani::assume(y == 0);
    assert_eq!(x + y, x);
}
