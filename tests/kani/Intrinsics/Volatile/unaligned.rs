// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Checks that `unaligned_volatile_load` and `unaligned_volatile_store` access
// memory byte-precisely through a deliberately misaligned pointer.
//
// These intrinsics carry no alignment requirement, so the codegen emits no
// alignment assertion for them. That makes the modelling itself the thing worth
// testing: each proof compares against a byte-wise oracle, so an implementation
// that quietly read or wrote the *aligned* word instead would be caught rather
// than silently pass. The oracle is built with `u32::from_ne_bytes` so the test
// is byte-precise without assuming an endianness: reading a `u32` at byte offset
// 1 must equal the native-order interpretation of bytes 1..5, whereas an
// alignment-assuming read from offset 0 would give the interpretation of bytes
// 0..4 -- different under either endianness.
#![feature(core_intrinsics)]

#[kani::proof]
fn check_unaligned_volatile_load_is_byte_precise() {
    let buf: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let p = unsafe { buf.as_ptr().add(1) } as *const u32;
    let v = unsafe { std::intrinsics::unaligned_volatile_load(p) };
    let expect = u32::from_ne_bytes([buf[1], buf[2], buf[3], buf[4]]);
    assert_eq!(v, expect, "unaligned load must read bytes 1..5, byte-precisely");
}

// The store direction: write a `u32` at byte offset 1 and check the neighbouring
// bytes too, so a wrongly-aligned write shows up as a clobbered neighbour rather
// than passing silently.
#[kani::proof]
fn check_unaligned_volatile_store_is_byte_precise() {
    let mut buf: [u8; 8] = [0u8; 8];
    let p = unsafe { buf.as_mut_ptr().add(1) } as *mut u32;
    let val: u32 = 0x55443322;
    let bytes = val.to_ne_bytes();
    unsafe { std::intrinsics::unaligned_volatile_store(p, val) };
    assert_eq!(buf[0], 0x00, "byte before the store must be untouched");
    assert_eq!(buf[1], bytes[0]);
    assert_eq!(buf[2], bytes[1]);
    assert_eq!(buf[3], bytes[2]);
    assert_eq!(buf[4], bytes[3]);
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
    let expect = u32::from_ne_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]]);
    assert_eq!(v, expect, "unaligned load must agree with a byte-wise oracle at every offset");
}

// A ZST store: `unaligned_volatile_store::<()>` must not attempt a dereference.
// A ZST pointer may legally be dangling-but-aligned, which is why
// `codegen_volatile_store` carries the same guard.
#[kani::proof]
fn check_zst_unaligned_volatile_store() {
    let mut zst = ();
    let p = &mut zst as *mut ();
    unsafe { std::intrinsics::unaligned_volatile_store(p, ()) };
}

// The case the ZST guard actually exists for: a *dangling* pointer. A ZST
// reference is allowed to be any non-null, suitably aligned address, so this is
// a pointer safe Rust can hand to the intrinsic. Nothing is dereferenced, so
// this must verify. The harness above passes a pointer to a real local, which a
// dereference would happily succeed on -- so it does not exercise the guard,
// and only this one does.
#[kani::proof]
fn check_dangling_zst_unaligned_volatile_store() {
    let p = core::ptr::without_provenance_mut::<()>(core::mem::align_of::<()>());
    unsafe { std::intrinsics::unaligned_volatile_store(p, ()) };
}
