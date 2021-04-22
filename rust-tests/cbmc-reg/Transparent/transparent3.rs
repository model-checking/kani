// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//#repr(transparent)]
pub struct Pointer<T> {
    pointer: *const T,
}

pub struct Container<T> {
    container: Pointer<T>,
}

fn main() {
    let x: u32 = 4;
    let my_pointer = Pointer { pointer: &x };
    let my_container = Container { container: my_pointer };

    let y: u32 = unsafe { *my_container.container.pointer };
    assert!(y == 4);
}
