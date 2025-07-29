// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we emit a nice error message for resolution failures.

/// Dummy structure
pub struct Bar;

/// Dummy stub
pub fn stub_foo() -> bool {
    true
}
/// Dummy trait
pub trait Foo {
    fn foo() -> bool {
        false
    }
}

impl Foo for Bar {}

// types don't impl Foo
#[kani::proof]
#[kani::stub(u8::foo, stub_foo)]
#[kani::stub(<(i32, i32)>::foo, stub_foo)]
#[kani::stub(<[u32]>::foo, stub_foo)]
#[kani::stub(str::foo, stub_foo)]
#[kani::stub(<[char; 10]>::foo, stub_foo)]
fn missing_impls() {}

// type impls Foo, but fn bar doesn't exist
#[kani::proof]
#[kani::stub(<Bar as Foo>::bar, stub_foo)]
fn invalid_methods() {}

trait A {
    fn bar(&self) -> u32;
}

trait B {
    fn bar(&self) -> u32;
}

struct X {}

impl A for X {
    fn bar(&self) -> u32 {
        200
    }
}

impl B for X {
    fn bar(&self) -> u32 {
        300
    }
}

// X::bar is ambiguous; two impls could match
#[kani::proof]
#[kani::stub(X::bar, A::bar)]
fn ambiguous_stub() {}

// B::bar doesn't have a body
#[kani::proof]
#[kani::stub(<X as A>::bar, B::bar)]
fn missing_body() {}
