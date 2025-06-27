// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
//! This test checks that contracts does not havoc
//! [promoted constants](https://github.com/rust-lang/const-eval/blob/master/promotion.md).
//! Related issue: <https://github.com/model-checking/kani/issues/3228>

extern crate kani;

#[derive(PartialEq, Eq, kani::Arbitrary)]
pub struct Foo(u8);

/// A named constant static should work the same way as a promoted constant.
static FOO: Foo = Foo(1);

/// A mutable static that should be havocked before contract validation.
static mut FOO_MUT: Foo = Foo(1);

/// Add a contract using a temporary variable that is lifted to a const static.
#[kani::requires(foo == Foo(1))]
pub fn foo_promoted(foo: Foo) -> Foo {
    assert!(foo.0 == 1);
    foo
}

/// Add a contract using a const static.
#[kani::requires(foo == FOO)]
pub fn foo_static(foo: Foo) -> Foo {
    assert!(foo.0 == 1);
    foo
}

/// Add a contract using a mutable static.
#[kani::requires(&foo == unsafe { &FOO_MUT })]
pub fn foo_mut_static(foo: Foo) -> Foo {
    assert!(foo.0 == 1);
    foo
}

#[kani::proof_for_contract(foo_promoted)]
fn check_promoted() {
    foo_promoted(kani::any());
}

#[kani::proof_for_contract(foo_static)]
fn check_static() {
    foo_static(kani::any());
}

#[kani::proof_for_contract(foo_mut_static)]
#[kani::should_panic]
fn check_mut_static() {
    foo_mut_static(kani::any());
}
