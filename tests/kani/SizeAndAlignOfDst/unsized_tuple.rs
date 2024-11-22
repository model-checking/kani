// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Ensure we compute the size correctly including padding for unsized tuple.
#![feature(unsized_tuple_coercion)]

use std::fmt::Debug;

#[kani::proof]
fn check_adjusted_tup_size() {
    let tup: (u32, [u8; 9]) = kani::any();
    let size = std::mem::size_of_val(&tup);

    let unsized_tup: *const (u32, dyn Debug) = &tup as *const _ as *const _;
    let adjusted_size = std::mem::size_of_val(unsafe { &*unsized_tup });

    assert_eq!(size, adjusted_size);
}
