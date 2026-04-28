// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test stubbing a method defined in a supertrait.

trait Base {
    fn base_method(&self) -> u32;
}

trait Derived: Base {
    fn derived_method(&self) -> u32;
}

struct MyType;

impl Base for MyType {
    fn base_method(&self) -> u32 {
        100
    }
}

impl Derived for MyType {
    fn derived_method(&self) -> u32 {
        200
    }
}

fn stub_base(_x: &MyType) -> u32 {
    1
}

fn stub_derived(_x: &MyType) -> u32 {
    2
}

#[kani::proof]
#[kani::stub(<MyType as Base>::base_method, stub_base)]
fn check_stub_supertrait_method() {
    let x = MyType;
    assert_eq!(x.base_method(), 1);
}

#[kani::proof]
#[kani::stub(<MyType as Derived>::derived_method, stub_derived)]
fn check_stub_subtrait_method() {
    let x = MyType;
    assert_eq!(x.derived_method(), 2);
    // Supertrait method should NOT be affected
    assert_eq!(x.base_method(), 100);
}
