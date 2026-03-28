// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stubbing a trait's default method implementation.
//!
//! FIXME: Default trait methods use `Self` in their body types, which
//! doesn't match the concrete type in the stub. The validation compares
//! the trait definition's body (with `&Self`) against the stub (with
//! `&MyType`), causing a false type mismatch.
//! Tracked in: https://github.com/model-checking/kani/issues/1997

trait HasDefault {
    fn default_method(&self) -> u32 {
        0
    }
}

struct MyType;
impl HasDefault for MyType {}

fn stub_default(_x: &MyType) -> u32 { 42 }

#[kani::proof]
#[kani::stub(<MyType as HasDefault>::default_method, stub_default)]
fn check_stub_default_trait_method() {
    let x = MyType;
    assert_eq!(x.default_method(), 42);
}
