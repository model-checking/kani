// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! A generic function that moves a value of its type parameter. The LLBC must move it, not copy
//! it: consumers such as Aeneas model ownership, and `T` need not be `Copy`. With rustc's MIR
//! optimizations, the move became a copy (`Option::Some { _0: copy x }`); LLBC runs now compile
//! at `-Zmir-opt-level=0`.

pub struct Slot<T> {
    value: Option<T>,
    count: u8,
}

pub fn put<T>(s: Slot<T>, x: T) -> Slot<T> {
    let Slot { value: _, count } = s;
    Slot { value: Some(x), count: count + 1 }
}

#[kani::proof]
fn main() {
    let s: Slot<u32> = Slot { value: None, count: 0 };
    let _ = put(s, 7);
}
