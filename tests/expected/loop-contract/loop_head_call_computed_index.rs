// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! A call in the loop body whose index is computed in the body (a binary-search-shaped loop
//! passing `s.get_unchecked(mid)` into a closure) must not have its initialization copied to
//! the loop head: at the head the computed index does not exist yet and `get_unchecked`'s
//! precondition check fails spuriously. The two sibling harnesses pin the shapes that already
//! worked: an element discarded with no binding, and the value copied out before the call.

#![feature(stmt_expr_attributes)]
#![feature(proc_macro_hygiene)]

use std::cmp::Ordering;

// The #4911 shape: computed index, element ref flows into the closure.
fn search<F: FnMut(&u8) -> Ordering>(s: &[u8], mut f: F) {
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
        let cmp = f(unsafe { s.get_unchecked(mid) });
        base = if cmp == Ordering::Greater { base } else { mid };
        size -= half;
    }
}

// Sibling pin 1: element discarded, no user binding starts a group.
fn search_no_closure(s: &[u8]) {
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
        let _ = unsafe { s.get_unchecked(mid) };
        base = if kani::any() { base } else { mid };
        size -= half;
    }
}

// Sibling pin 2: value copied out before the closure call.
fn search_copyval<F: FnMut(&u8) -> Ordering>(s: &[u8], mut f: F) {
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
        let v = unsafe { *s.get_unchecked(mid) };
        let cmp = f(&v);
        base = if cmp == Ordering::Greater { base } else { mid };
        size -= half;
    }
}

#[kani::proof]
fn check_with_closure() {
    let a: [u8; 8] = kani::any();
    let s: &[u8] = kani::slice::any_slice_of_array(&a);
    search(s, |_| if kani::any() { Ordering::Less } else { Ordering::Greater });
}

#[kani::proof]
fn check_no_closure() {
    let a: [u8; 8] = kani::any();
    let s: &[u8] = kani::slice::any_slice_of_array(&a);
    search_no_closure(s);
}

#[kani::proof]
fn check_copyval() {
    let a: [u8; 8] = kani::any();
    let s: &[u8] = kani::slice::any_slice_of_array(&a);
    search_copyval(s, |_| if kani::any() { Ordering::Less } else { Ordering::Greater });
}
