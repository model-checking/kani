// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we emit a nice error message for unsupported paths.

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

/// We still do not support stubbing for trait methods.
/// <https://github.com/model-checking/kani/issues/1997>
#[kani::proof]
#[kani::stub(<Bar as Foo>::foo, stub_foo)]
#[kani::stub(<Bar as Foo>::bar, stub_foo)]
#[kani::stub(<(i32, i32) as Foo>::foo, stub_foo)]
#[kani::stub(<[u32] as Foo>::foo, stub_foo)]
fn unsupported_args() {}
