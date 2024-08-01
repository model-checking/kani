// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Test that Kani can properly handle closure to fn ptr when an argument type is Never (`!`).
//! See <https://github.com/model-checking/kani/issues/3034> for more details.
#![feature(never_type)]

pub struct Foo {
    _x: i32,
    _never: !,
}

#[kani::proof]
fn check_unwrap_never() {
    let res = Result::<i32, Foo>::Ok(3);
    let _x = res.unwrap_or_else(|_f| 5);
}
