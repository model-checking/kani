// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

use std::ptr;
#[repr(C)]
struct S(u8, u16);

/// Checks that Kani catches an attempt to access padding of a struct using casting to different types.
#[kani::proof]
fn check_uninit_padding_after_cast() {
    unsafe {
        let mut s = S(0, 0);
        let sptr = ptr::addr_of_mut!(s);
        let sptr2 = sptr as *mut [u8; 4];
        *sptr2 = [0; 4];
        *sptr = S(0, 0); // should reset the padding
        let val = *sptr2; // ~ERROR: encountered uninitialized memory
    }
}
