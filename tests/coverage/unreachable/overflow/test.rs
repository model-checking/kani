// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn reduce(x: u32) -> u32 {
    if x > 1000 { x - 1000 } else { x }
}

#[kani::proof]
fn main() {
    reduce(7);
    reduce(33);
    reduce(728);
}
