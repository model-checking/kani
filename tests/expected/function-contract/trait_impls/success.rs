// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z function-contracts
//! Check that Kani supports function contracts on functions in trait implementations
//! using the motivating example from https://github.com/model-checking/kani/issues/1997

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

    #[kani::ensures(|res| *res == 0)]
    fn foo(&self) -> u32 {
        0
    }
}

impl A for X {
    #[kani::ensures(|res| *res == 100)]
    fn foo(&self) -> u32 {
        100
    }

    #[kani::ensures(|res| *res == 200)]
    fn bar(&self) -> u32 {
        200
    }
}

impl B for X {
    #[kani::ensures(|res| *res == 300)]
    fn bar(&self) -> u32 {
        300
    }
}

#[kani::proof_for_contract(<X as A>::foo)]
fn a_foo_harness() {
    let x = X::new();
    <X as A>::foo(&x);
}

#[kani::proof_for_contract(<X as A>::bar)]
fn a_bar_harness() {
    let x = X::new();
    <X as A>::bar(&x);
}

#[kani::proof_for_contract(<X as B>::bar)]
fn b_bar_harness() {
    let x = X::new();
    <X as B>::bar(&x);
}

#[kani::proof_for_contract(X::foo)]
fn x_harness() {
    let x = X::new();
    x.foo();
}
