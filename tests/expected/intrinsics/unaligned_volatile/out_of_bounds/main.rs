// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that an out-of-bounds access through `unaligned_volatile_load` and
// `unaligned_volatile_store` is still caught.
//
// The `unaligned_*` variants deliberately emit no alignment assertion, and they
// codegen the access as a bare dereference rather than going through place
// codegen, which is where Kani normally attaches validity assertions. That makes
// it worth pinning down that dereferenceability is nevertheless checked, by
// CBMC's `--pointer-check`. Every other test for these intrinsics uses a valid,
// in-bounds pointer, so none of them would notice if that check were absent.
//
// The pointer arithmetic here is in bounds -- `add(1)` on a four-byte array is
// well defined -- so it is the *access* that overruns the object: reading or
// writing a `u32` at byte offset 1 touches bytes 1..5 of a 4-byte allocation.
// That keeps the failure attributable to the dereference rather than to the
// offset computation.
#![feature(core_intrinsics)]

#[kani::proof]
fn check_unaligned_volatile_load_out_of_bounds() {
    let buf: [u8; 4] = [0x11, 0x22, 0x33, 0x44];
    let p = unsafe { buf.as_ptr().add(1) } as *const u32;
    let v = unsafe { std::intrinsics::unaligned_volatile_load(p) };
    assert_eq!(v, v);
}

#[kani::proof]
fn check_unaligned_volatile_store_out_of_bounds() {
    let mut buf: [u8; 4] = [0u8; 4];
    let p = unsafe { buf.as_mut_ptr().add(1) } as *mut u32;
    unsafe { std::intrinsics::unaligned_volatile_store(p, 0x55443322u32) };
    assert_eq!(buf[0], 0x00);
}
