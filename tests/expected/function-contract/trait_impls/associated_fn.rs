// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

//! Check that we can add contracts to associated functions.

extern crate kani;

#[derive(PartialEq, Eq)]
enum Foo {
    A(u8),
    B(char),
}

impl Foo {
    #[kani::ensures(|result| *result == Foo::A(inner))]
    pub fn new_a(inner: u8) -> Foo {
        Foo::A(inner)
    }

    #[kani::requires(char::from_u32(inner).is_some())]
    #[kani::ensures(|result| matches!(*result, Foo::B(c) if u32::from(c) == inner))]
    pub unsafe fn new_b(inner: u32) -> Foo {
        Foo::B(char::from_u32_unchecked(inner))
    }
}

#[kani::proof_for_contract(Foo::new_a)]
fn check_foo_a() {
    let _ = Foo::new_a(kani::any());
}

#[kani::proof_for_contract(Foo::new_b)]
fn check_foo_b() {
    let _ = unsafe { Foo::new_b(kani::any()) };
}
