// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// edition: 2021
// kani-flags: -Z uninit-checks

#[repr(C)]
struct Padded {
    byte: u8,
    word: u32,
}

#[kani::proof]
fn uninitialized_control() {
    let value = Padded { byte: 1, word: 2 };
    let pointer = std::ptr::addr_of!(value).cast::<u8>();
    let _padding = unsafe { *pointer.add(1) };
}
