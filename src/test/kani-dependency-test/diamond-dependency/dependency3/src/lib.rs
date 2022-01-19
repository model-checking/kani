// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub struct Foo {
    x: i32,
    // This field only in this version.
    y: i32,
}

// Export a function that takes a struct type which differs between this crate 
// and the other vesion.
pub fn take_foo(foo: &Foo) -> i32 {
    foo.x + foo.y
}

pub fn give_foo() -> Foo {
    Foo { x: 1, y: 2 }
}

pub fn get_int() -> i32 {
    // Use a constant to force an MIR GlobalAllocation::Memory
    let zero = &0;
    return *zero
}
