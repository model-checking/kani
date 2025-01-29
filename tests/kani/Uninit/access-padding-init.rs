// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

use std::ptr::addr_of;

#[repr(C)]
struct S(u32, u8);

#[kani::proof]
fn access_padding_init() {
    let s = S(0, 0);
    let ptr: *const u8 = addr_of!(s) as *const u8;
    let data = unsafe { *(ptr.add(3)) }; // Accessing data bytes is valid.
}
