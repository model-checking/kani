// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

use std::ptr::addr_of_mut;

#[repr(C)]
struct S(u32, u8);

#[kani::proof]
fn struct_padding_and_arr_init() {
    unsafe {
        let mut s = S(0, 0);
        let sptr = addr_of_mut!(s);
        let sptr2 = sptr as *mut [u8; 4];
        *sptr2 = [0; 4];
        *sptr = S(0, 0);
        // Both S(u32, u8) and [u8; 4] have the same layout, so the memory is initialized.
        let val = *sptr2;
    }
}
