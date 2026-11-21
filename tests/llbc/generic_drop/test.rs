// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zlean --print-llbc

//! A generic function that drops a value of its bare type parameter. Drop glue can't be resolved
//! for `T` itself, so the drop must point at the generic `drop_glue::<T>` declaration; resolving
//! it caused an internal compiler error.

pub fn first<T>(a: T, _b: T) -> T {
    a
}

#[kani::proof]
fn main() {
    let _ = first(1u32, 2u32);
}
