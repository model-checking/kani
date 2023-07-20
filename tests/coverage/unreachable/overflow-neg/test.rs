// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn negate(x: i32) -> i32 {
    if x != std::i32::MIN { -x } else { std::i32::MAX }
}

#[kani::proof]
fn main() {
    negate(std::i32::MIN);
}
