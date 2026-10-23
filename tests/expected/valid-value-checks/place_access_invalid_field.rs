// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z valid-value-checks
//! Narrowing a check to a field must still detect that field's invalid representation.

#[repr(C)]
struct Pair {
    first: bool,
    second: bool,
}

#[inline(never)]
unsafe fn read_second(ptr: *const Pair) -> bool {
    unsafe { (*ptr).second }
}

#[kani::proof]
fn invalid_bool_field_read() {
    let bytes = [1u8, 2];
    let ptr = (&raw const bytes).cast::<Pair>();
    let value = unsafe { read_second(ptr) };
    kani::cover!(value);
}
