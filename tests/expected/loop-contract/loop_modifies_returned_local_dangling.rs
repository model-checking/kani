// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z unstable-options --cbmc-args --object-bits 8

//! The loop adds `t` to its write set (it is declared in the loop body and a `return` reads it),
//! and Kani marks its storage live at the latch. A pointer to `t` that outlives the iteration is
//! still dangling, and the invariant that reads through it must still fail as a read of a dead
//! object.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

fn first_above(n: u8) -> u8 {
    let mut i: u8 = 0;
    let start: u8 = 0;
    let mut p: *const u8 = &start;
    #[kani::loop_invariant(i <= 5 && unsafe { *p } <= 5)]
    #[kani::loop_modifies(&i, &p)]
    while i < 5 {
        let t = i;
        if t > n {
            return t;
        }
        p = &t;
        i += 1;
    }
    0
}

#[kani::proof]
fn main() {
    let n: u8 = kani::any();
    let _ = first_above(n);
}
