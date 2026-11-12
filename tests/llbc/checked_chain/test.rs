// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! Several checked operations in one MIR block. Each is translated, as Charon does, to a call to
//! the `*_with_overflow` intrinsic that ends a block, so the MIR block is split once per
//! operation; Charon then folds each call and its overflow check into a panicking operation.

fn sum_product(a: u8, b: u8, c: u8) -> u8 {
    (a + b) * c - a
}

#[kani::proof]
fn main() {
    let _ = sum_product(1, 2, 3);
}
