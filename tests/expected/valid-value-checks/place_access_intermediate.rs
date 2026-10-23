// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Final stores and raw addresses must still validate intermediate reference loads.

// A store must still validate the intermediate reference used to locate its destination.
// Runtime MIR separates `**outer` into an intermediate reference load and a store. This
// exercises Read classification of that load, not multiple Deref projections in one Place.

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

// Forming a raw address can still require reading an invalid intermediate reference.
// Runtime MIR separates `**outer` into an intermediate reference load and a raw address.
// The load must retain its Read classification even though the final operation is RawAddress.

#[inline(never)]
unsafe fn address_through_reference(outer: *const &mut bool) -> *const bool {
    unsafe { &raw const **outer }
}

#[kani::proof]
fn invalid_intermediate_reference_raw_address() {
    let inner: *mut bool = std::ptr::null_mut();
    let outer = (&raw const inner).cast::<&mut bool>();
    let _address = unsafe { address_through_reference(outer) };
}
