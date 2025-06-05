// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub struct Foo {
    x: i32,
    // This field only in this version.
    y: i32,
}

pub enum Field {
    Case1,
    Case2,
}

#[repr(C)]
pub struct ReprCStruct {
    pub field: Field,
}

// Export a function that takes a struct type which differs between this crate
// and the other version.
pub fn take_foo(foo: &Foo) -> i32 {
    foo.x + foo.y
}

pub fn give_foo() -> Foo {
    Foo { x: 1, y: 2 }
}

pub fn get_int() -> i32 {
    // Use a constant to force an MIR GlobalAllocation::Memory
    let zero = &0;
    return *zero;
}
