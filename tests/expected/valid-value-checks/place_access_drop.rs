// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Ordinary assignment drops the old value before storing the replacement.

#[repr(transparent)]
struct Droppable {
    value: bool,
}

impl Drop for Droppable {
    #[inline(never)]
    fn drop(&mut self) {
        // Do not explicitly read value: this test concerns the value being dropped,
        // rather than an invalid bool load inside a user-written destructor.
    }
}

#[inline(never)]
unsafe fn replace(ptr: *mut Droppable) {
    unsafe { *ptr = Droppable { value: false } };
}

#[kani::proof]
fn invalid_old_value_drop() {
    let mut byte: u8 = 2;
    let ptr = (&raw mut byte).cast::<Droppable>();
    unsafe { replace(ptr) };
}
