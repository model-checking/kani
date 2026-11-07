// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Check a `#[kani::loop_modifies]` clause whose only target is a variable, such as a raw pointer
//! `p` or a mutable reference `t`. MIR optimizations used to merge the binding of such a clause
//! with that variable, which then had the name of the binding. When the variable is an argument,
//! the clause was dropped, and codegen skipped the writes through the variable, so
//! `mut_ref_argument_fail` and `write_after_loop_fail` verified. Kani panicked on a write through
//! the variable in the loop, and on a clause whose variable is a local.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

fn write_through(p: *mut u8) {
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies(p)]
    while kani::any() {
        unsafe { *p = 1 };
    }
}

#[kani::proof]
fn pointer_argument() {
    let mut x: u8 = 0;
    write_through(&raw mut x);
}

fn write_through_and_bad(p: *mut u8, bad: &mut u8) {
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies(p)]
    while kani::any() {
        unsafe { *p = 1 };
        *bad = 1;
    }
}

/// The loop writes `*bad`, which is not in its clause.
#[kani::proof]
fn pointer_argument_fail() {
    let mut x: u8 = 0;
    let mut bad: u8 = 0;
    write_through_and_bad(&raw mut x, &mut bad);
}

fn count(t: &mut u8) {
    let mut i: u8 = 0;
    #[kani::loop_invariant(i <= 2)]
    #[kani::loop_modifies(t)]
    while i < 2 {
        i += 1;
    }
}

/// The loop writes `i`, which is not in its clause.
#[kani::proof]
fn mut_ref_argument_fail() {
    let mut x: u8 = 0;
    count(&mut x);
}

fn clear(pp: *mut *mut u8) {
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies(pp)]
    while kani::any() {}
    unsafe { *pp = core::ptr::null_mut() };
}

/// `clear` sets `q` to null after its loop, so the assertion fails.
#[kani::proof]
fn write_after_loop_fail() {
    let mut x: u8 = 0;
    let mut q: *mut u8 = &raw mut x;
    clear(&raw mut q);
    assert!(!q.is_null());
}

#[kani::proof]
fn pointer_local() {
    let mut x: u8 = 0;
    let p = &raw mut x;
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies(p)]
    while kani::any() {
        unsafe { *p = 1 };
    }
}

/// The loop writes `bad`, which is not in its clause.
#[kani::proof]
fn pointer_local_fail() {
    let mut x: u8 = 0;
    let mut bad: u8 = 0;
    let p = &raw mut x;
    #[kani::loop_invariant(true)]
    #[kani::loop_modifies(p)]
    while kani::any() {
        unsafe { *p = 1 };
        bad = 1;
    }
    let _ = bad;
}
