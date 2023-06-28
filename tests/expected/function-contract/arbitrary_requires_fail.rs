// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

fn div(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

#[kani::proof]
fn main() {
    div(kani::any(), kani::any());
}
