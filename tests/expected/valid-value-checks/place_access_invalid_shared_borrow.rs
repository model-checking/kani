// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Only create a reference and observe its address; do not read the bool through it.

#[inline(never)]
unsafe fn borrow_bool<'a>(ptr: *const bool) -> &'a bool {
    unsafe { &*ptr }
}

#[kani::proof]
fn invalid_bool_shared_borrow() {
    let byte: u8 = 2;
    let ptr = (&raw const byte).cast::<bool>();
    let reference = unsafe { borrow_bool(ptr) };
    assert!(!std::ptr::from_ref(reference).is_null());
}
