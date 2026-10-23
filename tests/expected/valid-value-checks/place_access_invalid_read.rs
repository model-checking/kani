// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Initialized bytes with an invalid bool representation, not uninitialized memory.

#[inline(never)]
unsafe fn read_bool(ptr: *const bool) -> bool {
    unsafe { *ptr }
}

#[kani::proof]
fn invalid_bool_read() {
    let byte: u8 = 2;
    let ptr = (&raw const byte).cast::<bool>();
    let value = unsafe { read_bool(ptr) };
    kani::cover!(value);
}
