// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stubbing trait method implementations using fully-qualified syntax.
//! Regression test for https://github.com/model-checking/kani/issues/1997

trait A {
    fn foo(&self) -> u32;
    fn bar(&self) -> u32;
}

trait B {
    fn bar(&self) -> u32;
}

struct X {}

impl X {
    fn new() -> Self {
        Self {}
    }
}

impl A for X {
    fn foo(&self) -> u32 {
        100
    }
    fn bar(&self) -> u32 {
        200
    }
}

impl B for X {
    fn bar(&self) -> u32 {
        300
    }
}

fn stub_1(_x: &X) -> u32 {
    1
}
fn stub_2(_x: &X) -> u32 {
    2
}
fn stub_3(_x: &X) -> u32 {
    3
}

#[kani::proof]
#[kani::stub(<X as A>::foo, stub_1)]
fn check_stub_trait_a_foo() {
    let x = X::new();
    assert_eq!(x.foo(), 1);
}

#[kani::proof]
#[kani::stub(<X as A>::bar, stub_2)]
fn check_stub_trait_a_bar() {
    let x = X::new();
    assert_eq!(A::bar(&x), 2);
}

#[kani::proof]
#[kani::stub(<X as B>::bar, stub_3)]
fn check_stub_trait_b_bar() {
    let x = X::new();
    assert_eq!(<X as B>::bar(&x), 3);
}
