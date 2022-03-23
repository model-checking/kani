// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Check that `volatile_store` passes when it writes to an aligned value.
// This example is similar to the one that appears in the documentation for
// `write_unaligned`:
// https://doc.rust-lang.org/std/ptr/fn.write_unaligned.html
#![feature(core_intrinsics)]

// In contrast to the `Packed` struct in `store_fail.rs`, this struct includes
// padding so that each field is aligned.
struct NonPacked {
    _padding: u8,
    unaligned: u32,
}

#[kani::proof]
fn main() {
    let mut packed: NonPacked = unsafe { std::mem::zeroed() };
    // Take the address of a 32-bit integer which is not aligned.
    // In contrast to `&packed.unaligned as *mut _`, this has no undefined behavior.
    let unaligned = std::ptr::addr_of_mut!(packed.unaligned);

    // Store the value with `volatile_store`.
    // This includes an alignment check for `unaligned` which should pass.
    unsafe { std::intrinsics::volatile_store(unaligned, 42) };
    assert!(packed.unaligned == 42);
}
