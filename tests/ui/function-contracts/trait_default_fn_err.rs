// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts
//
//! This tests that we emit a nice error message when a user tries
//! to apply a contract to a default trait fn without a body.

struct Bar;

pub trait Foo {
    #[kani::requires(true)]
    fn foo(&self) -> bool;
}

impl Foo for Bar {
    fn foo(&self) -> bool {
        true
    }
}

#[kani::proof]
fn harness() {}
