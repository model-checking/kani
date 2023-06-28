// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn div(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

#[kani::proof]
fn main() {
    div(kani::any(), kani::any());
}
