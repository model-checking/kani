// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn rem(x: u16, y: u16) -> u16 {
    if y != 0 { x % y } else { 0 }
}

#[kani::proof]
fn main() {
    rem(5, 0);
}
