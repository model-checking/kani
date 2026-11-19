// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! This test checks that Kani's LLBC backend handles integer literals of every width, signed and
//! unsigned.

fn signed() -> (i8, i16, i32, i64, i128, isize) {
    (-1, -2, -3, -4, -5, -6)
}
fn unsigned() -> (u8, u16, u32, u64, u128, usize) {
    (1, 2, 3, 4, 5, 6)
}
#[kani::proof]
fn main() {
    let _ = signed();
    let _ = unsigned();
}
