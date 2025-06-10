// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can stub trait implementations.

/// Dummy structure
pub struct Bar;

/// Dummy trait
pub trait Foo {
    fn foo() -> bool {
        false
    }
}

impl Foo for Bar {}

impl Foo for u8 {}

impl<T> Foo for [T] {}

impl Foo for [char; 10] {}

impl Foo for (i32, i32) {}

/// Dummy stub
pub fn stub_foo() -> bool {
    true
}

#[kani::proof]
#[kani::stub(<Bar as Foo>::foo, stub_foo)]
#[kani::stub(<(i32, i32) as Foo>::foo, stub_foo)]
#[kani::stub(<[u32] as Foo>::foo, stub_foo)]
fn stub_trait_methods() {}
