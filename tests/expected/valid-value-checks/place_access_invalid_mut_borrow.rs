// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Regression for incorrectly skipping all PlaceContext::MUTATING accesses.

#[inline(never)]
unsafe fn borrow_bool<'a>(ptr: *mut bool) -> &'a mut bool {
    unsafe { &mut *ptr }
}

#[kani::proof]
fn invalid_bool_mut_borrow() {
    let mut byte: u8 = 2;
    // u8 has no restricted bit patterns: this cast must not mask the missing borrow check
    // with a mutable-cast UnsupportedCheck.
    let ptr = (&raw mut byte).cast::<bool>();
    let reference = unsafe { borrow_bool(ptr) };
    assert!(!std::ptr::from_mut(reference).is_null());
}
