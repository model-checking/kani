// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks

#[repr(C)]
struct Inner {
    valid: bool,
    pending: bool,
}

#[repr(C)]
struct Outer {
    inner: Inner,
    other: bool,
}

#[inline(never)]
unsafe fn borrow_nested<'a>(ptr: *mut Outer) -> &'a mut bool {
    unsafe { &mut (*ptr).inner.pending }
}

#[kani::proof]
fn invalid_nested_mut_borrow() {
    // repr(C) puts the invalid bool at byte offset one; all bytes are initialized.
    let mut bytes = [1u8, 2, 1];
    let ptr = (&raw mut bytes).cast::<Outer>();
    let reference = unsafe { borrow_nested(ptr) };
    assert!(!std::ptr::from_mut(reference).is_null());
}
