// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! At `-Zmir-opt-level=0`, MIR passes values of zero-sized types such as `PhantomData` as
//! zero-sized constants, which the backend translates as aggregates without fields.

use std::marker::PhantomData;

pub struct Tagged<T> {
    pub tag: PhantomData<T>,
    pub n: u8,
}

pub fn tagged<T>(n: u8) -> Tagged<T> {
    Tagged { tag: PhantomData, n }
}

#[kani::proof]
fn main() {
    let _: Tagged<u32> = tagged(3);
}
