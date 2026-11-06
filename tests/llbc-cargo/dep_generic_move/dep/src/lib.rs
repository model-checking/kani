// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A generic function in a dependency. The LLBC backend translates its body from the
//! dependency's metadata, so the dependency must be compiled at the same MIR level as the local
//! crate: otherwise the move of `x` is a copy.

pub struct Slot<T> {
    pub value: Option<T>,
    pub count: u8,
}

pub fn put<T>(s: Slot<T>, x: T) -> Slot<T> {
    let Slot { value: _, count } = s;
    Slot { value: Some(x), count: count + 1 }
}
