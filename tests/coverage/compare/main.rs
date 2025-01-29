// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn compare(x: u16, y: u16) -> u16 {
    // The case where `x < y` isn't possible so its region is `UNCOVERED`
    if x >= y { 1 } else { 0 }
}

#[kani::proof]
fn main() {
    let x: u16 = kani::any();
    let y: u16 = kani::any();
    if x >= y {
        compare(x, y);
    }
}
