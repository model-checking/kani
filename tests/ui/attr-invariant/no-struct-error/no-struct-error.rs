// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Invariant` for structs with named fields.

extern crate kani;
use kani::Invariant;

#[kani::invariant(true)]
enum MyEnum {
    A,
    B(i32),
}
