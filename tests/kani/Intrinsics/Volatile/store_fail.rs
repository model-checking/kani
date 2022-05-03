// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

// Check that `volatile_store` fails when it writes to an unaligned value.
// This example is similar to the one that appears in the documentation for
// `write_unaligned`:
// https://doc.rust-lang.org/std/ptr/fn.write_unaligned.html
#![feature(core_intrinsics)]

// `repr(packed)` forces the struct to be stripped of any padding and only align
// its fields to a byte.
#[repr(packed)]
struct Packed {
    _padding: u8,
    unaligned: u32,
}

#[kani::proof]
fn main() {
    let mut packed: Packed = unsafe { std::mem::zeroed() };
    // Take the address of a 32-bit integer which is not aligned.
    // In contrast to `&packed.unaligned as *mut _`, this has no undefined behavior.
    let unaligned = std::ptr::addr_of_mut!(packed.unaligned);

    // Store the value with `volatile_store`.
    // This includes an alignment check for `unaligned` which should fail.
    unsafe { std::intrinsics::volatile_store(unaligned, 42) };
    assert!(packed.unaligned == 42);
}
