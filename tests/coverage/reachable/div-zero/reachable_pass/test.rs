// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn div(x: u16, y: u16) -> u16 {
    if y != 0 { x / y } else { 0 } // PARTIAL: some cases are `COVERED`, others are not
}

#[kani::proof]
fn main() {
    div(11, 3);
}
