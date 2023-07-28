// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn rem(x: u16, y: u16) -> u16 {
    x % y // PARTIAL: `x % y` is covered but induces a division failure
} // NONE: Caused by division failure earlier

#[kani::proof]
fn main() {
    rem(678, 0);
} // NONE: Caused by division failure earlier
