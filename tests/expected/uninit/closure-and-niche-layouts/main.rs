// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Layouts of closures with captures, and of enums whose niche-encoded value is one scalar.

use std::mem::MaybeUninit;
use std::num::NonZero;

fn call_through_pointer<F: Fn() -> u64>(f: &F) -> u64 {
    let p = f as *const F;
    unsafe { (*p)() }
}

#[kani::proof]
fn closure_with_captures_ok() {
    let a: u32 = kani::any();
    let b: u8 = kani::any();
    let f = || a as u64 + b as u64;
    assert_eq!(call_through_pointer(&f), a as u64 + b as u64);
}

#[kani::proof]
fn closure_capturing_uninit_ub() {
    let a = MaybeUninit::<u64>::uninit();
    let f = move || unsafe { a.assume_init() };
    let _ = call_through_pointer(&f);
}

/// `NonZero::new` transmutes `usize` into `Option<NonZero<usize>>`.
#[kani::proof]
fn transmute_into_niche_enum_ok() {
    let n: usize = kani::any();
    assert_eq!(NonZero::new(n).is_some(), n != 0);
}
