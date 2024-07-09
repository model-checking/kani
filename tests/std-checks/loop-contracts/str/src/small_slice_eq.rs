// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! https://github.com/rust-lang/rust/blob/7c4ac0603e9ee5295bc802c90575391288a69a8a/library/core/src/str/pattern.rs#L1881

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]
#![feature(ptr_sub_ptr)]
unsafe fn small_slice_eq(x: &[u8], y: &[u8]) -> bool {
    debug_assert_eq!(x.len(), y.len());

    if x.len() < 4 {
        for (&b1, &b2) in x.iter().zip(y) {
            if b1 != b2 {
                return false;
            }
        }
        return true;
    }

    unsafe {
        let (mut px, mut py) = (x.as_ptr(), y.as_ptr());
        let (pxend, pyend) = (px.add(x.len() - 4), py.add(y.len() - 4));
        // #[kani::loop_invariant( px as isize >= x.as_ptr() as isize
        // && py as isize >= y.as_ptr() as isize
        // && px as isize - x.as_ptr() as isize == (py as isize - y.as_ptr() as isize))]
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
fn main() {
    let mut a = [1; 20];
    let mut b = [1; 20];
    unsafe {
        small_slice_eq(&mut a, &mut b);
    }
}
