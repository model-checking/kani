// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! This test checks that contracts does not havoc
//! [promoted constants](https://github.com/rust-lang/const-eval/blob/master/promotion.md)
//! that represents an enum variant.
//!
//! Related issue: <https://github.com/model-checking/kani/issues/3228>

extern crate kani;
#[derive(PartialEq, Eq, kani::Arbitrary)]
pub enum Foo {
    A,
    B,
}

#[kani::ensures(|result: &Foo| *result == Foo::A)]
pub fn foo_a() -> Foo {
    Foo::A
}

#[kani::proof_for_contract(foo_a)]
fn check() {
    let _ = foo_a();
}

#[kani::proof]
#[kani::stub_verified(foo_a)]
fn check_stub() {
    let val = foo_a();
    assert!(val == Foo::A)
}
