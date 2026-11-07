// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts

//! Signed companion to `for_loop_symbolic_range.rs`: a `for` loop over a fully
//! symbolic `Range<i8>` (which may be empty, and may span more than `i8::MAX`
//! elements) must count correctly. This exercises both the `abs_diff` path of
//! the `Range` model's `len()` and the `wrapping_add` in `nth`, where
//! `start + i as i8` would otherwise overflow for a wide signed span.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::proof]
fn for_loop_symbolic_range_signed() {
    // `start <= end` is NOT assumed: the range may be empty (start > end).
    let start: i8 = kani::any();
    let end: i8 = kani::any();

    let mut count: usize = 0;
    #[kani::loop_invariant(count == kani::index)]
    for _i in start..end {
        count += 1;
    }
    let native = if end > start { (end as i16 - start as i16) as usize } else { 0 };
    assert!(count == native);
}
