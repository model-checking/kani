// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub struct Foo {
    x: i32,
}

// Export a function that takes a struct type which differs between this crate 
// and the other vesion
pub fn take_foo(foo: &Foo) -> i32 {
    foo.x
}

pub fn give_foo() -> Foo {
    Foo { x: 1 }
}

pub fn get_int() -> i32 {
    // Use a constant to force an MIR GlobalAllocation::Memory.
    // Use a non-i32 so there will be a conflict between this
    // version and the other version. The constant is also a 
    // different value than the other version of this dependency.
    let one = &(1 as i8);
    return *one as i32
}
