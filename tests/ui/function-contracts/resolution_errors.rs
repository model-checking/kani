// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Zfunction-contracts
//
//! This tests that we emit a nice error message for resolution failures.

/// Dummy structure
pub struct Bar;

/// Dummy trait
pub trait Foo {
    #[kani::ensures(|res| *res)]
    fn foo() -> bool {
        false
    }
}

// types don't impl Foo
#[kani::proof_for_contract(u8::foo)]
fn missing_impl_u8() {}

#[kani::proof_for_contract(<(i32, i32)>::foo)]
fn missing_impl_tuple() {}

#[kani::proof_for_contract(<[u32]>::foo)]
fn missing_impl_slice() {}

#[kani::proof_for_contract(str::foo)]
fn missing_impl_str() {}

#[kani::proof_for_contract(<[char; 10]>::foo)]
fn missing_impl_array() {}

// type impls Foo, but fn bar doesn't exist
#[kani::proof_for_contract(<Bar as Foo>::bar)]
fn invalid_method() {}

trait A {
    fn bar(&self) -> u32;
}

trait B {
    fn bar(&self) -> u32;
}

struct X {}

impl A for X {
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

// X::bar is ambiguous; two impls could match
#[kani::proof_for_contract(X::bar)]
fn ambiguous_method() {}
