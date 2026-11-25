// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[repr(C)]
struct Bytes {
    a: u8,
    b: u32,
}

fn uninit_read() {
    let bytes = Bytes { a: 7, b: 7 };
    // Offset 1 is padding between the two fields.
    let ptr = (&raw const bytes).cast::<u8>();
    let _padding = unsafe { ptr.add(1).read() };
}

fn initialized_read() {
    let mut bytes = Bytes { a: 7, b: 7 };
    let ptr = (&raw mut bytes).cast::<u8>();
    // Initialize the padding byte in place before reading it.
    unsafe { ptr.add(1).write(7) };
    let _padding = unsafe { ptr.add(1).read() };
}
