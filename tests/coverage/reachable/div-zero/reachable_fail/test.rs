// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn div(x: u16, y: u16) -> u16 {
    x / y
}

#[kani::proof]
fn main() {
    div(678, 0);
}
