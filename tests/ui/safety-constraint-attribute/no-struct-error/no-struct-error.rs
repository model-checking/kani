// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani raises an error when a derive macro with the
//! `#[safety_constraint(...)]` attribute is is used in items which are not a
//! struct.
//!
//! Note: the `#[derive(kani::Invariant)]` macro is required for the compiler error,
//! otherwise the `#[safety_constraint(...)]` attribute is ignored.

extern crate kani;

#[derive(kani::Invariant)]
#[safety_constraint(true)]
enum MyEnum {
    A,
    B(i32),
}
