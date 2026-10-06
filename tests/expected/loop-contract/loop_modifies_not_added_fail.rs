// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check that the locals Kani adds to a `#[kani::loop_modifies]` clause do not hide other
//! failures: a variable declared before the loop is not added even if the loop writes it before
//! reading it, and adding a variable declared in the loop body does not extend its lifetime.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

/// `y` is declared before the loop and not in the clause, so the assigns check for `y` must fail.
#[kani::proof]
fn outer_written_before_read() {
    let mut i: u8 = 0;
    let mut y: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i)]
    while i < 2 {
        y = 5;
        i += 1;
    }
    let _ = y;
}

/// `x` is dead when `p` is dereferenced, so the dereference must fail.
#[kani::proof]
fn body_local_used_after_scope() {
    let mut i: u8 = 0;
    let mut p: *const u8 = std::ptr::null();
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(&i, &p)]
    while i < 2 {
        {
            let x: u8 = 7;
            p = &raw const x;
        }
        let v = unsafe { *p };
        assert!(v == 7);
        i += 1;
    }
}
