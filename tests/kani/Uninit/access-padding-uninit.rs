// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

use std::ptr::addr_of;

#[repr(C)]
struct S(u32, u8);

#[kani::proof]
fn main() {
    let s = S(0, 0);
    let ptr: *const u8 = addr_of!(s) as *const u8;
    let padding = unsafe { *(ptr.add(5)) };
}

