// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Forming a raw address can still require reading an invalid intermediate reference.
//! Runtime MIR separates `**outer` into an intermediate reference load and a raw address.
//! The load must retain its Read classification even though the final operation is RawAddress.

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
