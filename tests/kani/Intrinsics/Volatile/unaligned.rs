// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `unaligned_volatile_load` and `unaligned_volatile_store` access
// memory byte-precisely through a deliberately misaligned pointer.
//
// These intrinsics carry no alignment requirement, so the codegen emits no
// alignment assertion for them. That makes the modelling itself the thing worth
// testing: each proof compares against a byte-wise oracle, so an implementation
// that quietly read or wrote the *aligned* word instead would be caught rather
// than silently pass. Reading a `u32` at byte offset 1 of the pattern below, the
// only correct little-endian answer is 0x55443322; an alignment-assuming read
// from offset 0 would give 0x44332211.
#![feature(core_intrinsics)]

#[kani::proof]
fn check_unaligned_volatile_load_is_byte_precise() {
    let buf: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let p = unsafe { buf.as_ptr().add(1) } as *const u32;
    let v = unsafe { std::intrinsics::unaligned_volatile_load(p) };
    assert_eq!(v, 0x55443322u32, "unaligned load must read bytes 1..5, byte-precisely");
}

// The store direction: write a `u32` at byte offset 1 and check the neighbouring
// bytes too, so a wrongly-aligned write shows up as a clobbered neighbour rather
// than passing silently.
#[kani::proof]
fn check_unaligned_volatile_store_is_byte_precise() {
    let mut buf: [u8; 8] = [0u8; 8];
    let p = unsafe { buf.as_mut_ptr().add(1) } as *mut u32;
    unsafe { std::intrinsics::unaligned_volatile_store(p, 0x55443322u32) };
    assert_eq!(buf[0], 0x00, "byte before the store must be untouched");
    assert_eq!(buf[1], 0x22);
    assert_eq!(buf[2], 0x33);
    assert_eq!(buf[3], 0x44);
    assert_eq!(buf[4], 0x55);
    assert_eq!(buf[5], 0x00, "byte after the store must be untouched");
}

// Symbolic offset: the offset is not a constant, so the result cannot be reached
// by constant folding. This is the case that would expose an alignment
// assumption hidden behind constant propagation.
#[kani::proof]
fn check_unaligned_volatile_load_symbolic_offset() {
    let buf: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let off: usize = kani::any();
    kani::assume(off <= 4);
    let p = unsafe { buf.as_ptr().add(off) } as *const u32;
    let v = unsafe { std::intrinsics::unaligned_volatile_load(p) };
    let expect = u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
    assert_eq!(v, expect, "unaligned load must agree with a byte-wise oracle at every offset");
}
