// Copyright rustc Contributors
// Adapted from rust std: https://github.com/rust-lang/rust/blob/master/library/core/src/str/pattern.rs#L1885
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// kani-flags: -Z loop-contracts --enable-unstable --cbmc-args --arrays-uf-always --no-standard-checks --object-bits 8

//! Check if loop contracts are correctly applied. The flag --no-standard-checks should be
//! removed once same_object predicate is supported in loop contracts.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(ptr_sub_ptr)]
unsafe fn small_slice_eq(x: &[u8], y: &[u8]) -> bool {
    debug_assert_eq!(x.len(), y.len());
    unsafe {
        let (mut px, mut py) = (x.as_ptr(), y.as_ptr());
        let (pxend, pyend) = (px.add(x.len() - 4), py.add(y.len() - 4));
        #[kani::loop_invariant( px as isize >= x.as_ptr() as isize
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
    // Needed to avoid having `free` be removed as unused function. This is
    // because DFCC contract enforcement assumes that a definition for `free`
    // exists.
    let _ = Box::new(10);
    let mut a = [1; 2000];
    let mut b = [1; 2000];
    unsafe {
        small_slice_eq(&mut a, &mut b);
    }
}
