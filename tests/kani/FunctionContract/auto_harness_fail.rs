
// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#[kani::ensures(result == x)]
#[kani::auto_harness]
fn max(x: u32, y: u32) -> u32 {
    if x > y {
        x
    } else {
        y
    }
}
