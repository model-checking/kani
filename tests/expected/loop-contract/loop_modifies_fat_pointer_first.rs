// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A user-written `loop_modifies` clause whose FIRST target is a fat pointer. Codegen paired the
//! clause's targets with the modifies tuple's fields in struct-field order; layout reorders the
//! fields around the fat pointer, so this spelling crashed at expr.rs:907. Codegen'ing each
//! operand directly keeps the pairing in operand order.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn fat_pointer_first() {
    let mut a: [u8; 4] = [0; 4];
    let mut i: usize = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 4)]
    #[kani::loop_modifies(core::ptr::slice_from_raw_parts(a.as_ptr(), 4), &i, &j)]
    while i < 4 {
        a[i] = j;
        j = j.wrapping_add(1);
        i += 1;
    }
}

// Non-regression: the fat pointer in second position matches the layout and already worked;
// the per-operand rewrite must keep it working.
#[kani::proof]
fn fat_pointer_second() {
    let mut a: [u8; 4] = [0; 4];
    let mut i: usize = 0;
    let mut j: u8 = 0;
    #[kani::loop_invariant(i <= 4)]
    #[kani::loop_modifies(&i, core::ptr::slice_from_raw_parts(a.as_ptr(), 4), &j)]
    while i < 4 {
        a[i] = j;
        j = j.wrapping_add(1);
        i += 1;
    }
}
