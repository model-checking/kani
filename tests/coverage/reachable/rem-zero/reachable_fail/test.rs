// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn rem(x: u16, y: u16) -> u16 {
    x % y
}

#[kani::proof]
fn main() {
    rem(678, 0);
}
