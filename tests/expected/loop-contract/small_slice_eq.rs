// Copyright rustc Contributors
// Adapted from rust std: https://github.com/rust-lang/rust/blob/master/library/core/src/str/pattern.rs#L1885
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// kani-flags: -Z loop-contracts -Z mem-predicates --enable-unstable --cbmc-args --object-bits 8

//! Check if loop contracts are correctly applied.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(ptr_sub_ptr)]

extern crate kani;

use kani::mem::same_allocation;

unsafe fn small_slice_eq(x: &[u8], y: &[u8]) -> bool {
    debug_assert_eq!(x.len(), y.len());
    unsafe {
        let (mut px, mut py) = (x.as_ptr(), y.as_ptr());
        let (pxend, pyend) = (px.add(x.len() - 4), py.add(y.len() - 4));
        #[kani::loop_invariant( same_allocation(x.as_ptr(), px) && same_allocation(y.as_ptr(), py)
        && px as isize >= x.as_ptr() as isize
        && py as isize >= y.as_ptr() as isize
        && px as isize - x.as_ptr() as isize == (py as isize - y.as_ptr() as isize))]
        while px < pxend {
            let vx = (px as *const u32).read_unaligned();
            let vy = (py as *const u32).read_unaligned();
            if vx != vy {
                return false;
            }
            px = px.add(4);
            py = py.add(4);
        }
        let vx = (pxend as *const u32).read_unaligned();
        let vy = (pyend as *const u32).read_unaligned();
        vx == vy
    }
}

#[kani::proof]
fn small_slice_eq_harness() {
    const ARR_SIZE: usize = 2000;
    let x: [u8; ARR_SIZE] = kani::any();
    let y: [u8; ARR_SIZE] = kani::any();
    unsafe {
        small_slice_eq(&x, &y);
    }
}
