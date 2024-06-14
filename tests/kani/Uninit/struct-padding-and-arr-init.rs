// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

#![feature(core_intrinsics)]
#![feature(custom_mir)]

use std::intrinsics::mir::*;
use std::ptr;

#[repr(C)]
struct S(u16, u16);

#[kani::proof]
#[custom_mir(dialect = "runtime", phase = "optimized")]
fn main() {
    mir! {
        let s: S;
        let sptr;
        let sptr2;
        let _val;
        {
            sptr = ptr::addr_of_mut!(s);
            sptr2 = sptr as *mut [u8; 4];
            *sptr2 = [0; 4];
            *sptr = S(0, 0);
            // Both S(u16, u16) and [u8; 4] have the same layout, so the memory is initialized.
            _val = *sptr2; 
            Return()
        }
    }
}
