// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani raises an error when the `kani::invariant` attribute is
//! applied to items which is not a struct.

extern crate kani;

#[kani::invariant(true)]
enum MyEnum {
    A,
    B(i32),
}
