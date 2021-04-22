// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[repr(transparent)]
pub struct Pointer<T> {
    pointer: *const T,
}

#[repr(transparent)]
pub struct Wrapper<T>(T);

pub struct Container<T> {
    container: Pointer<T>,
}

fn main() {
    let x: u32 = 4;
    let my_container = Container { container: Pointer { pointer: &x } };

    let y: u32 = unsafe { *my_container.container.pointer };
    assert!(y == 4);

    let w: Wrapper<u32> = Wrapper(4);

    let Wrapper(c) = w;
    assert!(c == 4);
}
