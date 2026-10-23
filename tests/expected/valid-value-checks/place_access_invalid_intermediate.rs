// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! A store must still validate the intermediate reference used to locate its destination.
//! Runtime MIR separates `**outer` into an intermediate reference load and a store. This
//! exercises Read classification of that load, not multiple Deref projections in one Place.

#[inline(never)]
unsafe fn store_through_reference(outer: *mut &mut bool) {
    unsafe { **outer = true };
}

#[kani::proof]
fn invalid_intermediate_reference() {
    // The raw null pointer is a valid, initialized value. Interpreting these bytes as a
    // reference is invalid. This does not rely on validity checks detecting uninitialized data.
    let mut inner: *mut bool = std::ptr::null_mut();
    let outer = (&raw mut inner).cast::<&mut bool>();
    unsafe { store_through_reference(outer) };
}
