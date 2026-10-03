// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Negative twin of loop_head_call_computed_index.rs: with the spurious loop-head copy gone, a
//! genuinely out-of-bounds `get_unchecked` in the loop body must still fail its precondition
//! check at the real call site.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

use std::cmp::Ordering;

fn search_oob<F: FnMut(&u8) -> Ordering>(s: &[u8], mut f: F) {
    let len = s.len();
    if len == 0 {
        return;
    }
    let mut size = len;
    let mut base = 0usize;
    #[kani::loop_invariant(1 <= size && size <= len && base <= len - size)]
    while size > 1 {
        let half = size / 2;
        let mid = base + half;
        // mid + 1 can equal len: a real precondition violation at the body call site.
        let cmp = f(unsafe { s.get_unchecked(mid + 1) });
        base = if cmp == Ordering::Greater { base } else { mid };
        size -= half;
    }
}

#[kani::proof]
fn check_oob_still_fails() {
    let a: [u8; 8] = kani::any();
    let s: &[u8] = kani::slice::any_slice_of_array(&a);
    search_oob(s, |_| if kani::any() { Ordering::Less } else { Ordering::Greater });
}
