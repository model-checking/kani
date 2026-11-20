// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Unions nested in other types, and raw pointers to unions, under `-Z uninit-checks`
//! (https://github.com/model-checking/kani/issues/5151). The `_ok` harnesses must verify; the
//! `_ub` harnesses read uninitialized memory and must be reported.

use std::mem::MaybeUninit;
use std::ptr;
use std::ptr::addr_of;

struct D(u8);
impl Drop for D {
    fn drop(&mut self) {}
}

/// The reproducer from the issue: slot 1 was never written.
#[kani::proof]
fn drop_uninit_in_array_ub() {
    let mut a: [MaybeUninit<D>; 2] = [const { MaybeUninit::uninit() }; 2];
    a[0].write(D(1));
    unsafe { ptr::drop_in_place(a.as_mut_ptr().cast::<D>().add(1)) };
}

/// Dropping the slot that was written is fine.
#[kani::proof]
fn drop_init_in_array_ok() {
    let mut a: [MaybeUninit<D>; 2] = [const { MaybeUninit::uninit() }; 2];
    a[0].write(D(1));
    unsafe { ptr::drop_in_place(a.as_mut_ptr().cast::<D>()) };
}

/// A buffer initialized element by element and read back through a raw pointer.
#[kani::proof]
fn buffer_written_then_read_ok() {
    let mut buffer = [MaybeUninit::<u32>::uninit(); 4];
    for slot in buffer.iter_mut() {
        *slot = MaybeUninit::new(kani::any());
    }
    let i: usize = kani::any_where(|i| *i < 4);
    let base = buffer.as_ptr() as *const u32;
    let _ = unsafe { ptr::read(base.add(i)) };
}

/// Only some elements were written; reading another one is reported.
#[kani::proof]
fn buffer_partly_written_ub() {
    let mut buffer = [MaybeUninit::<u32>::uninit(); 4];
    buffer[0] = MaybeUninit::new(1);
    let base = buffer.as_ptr() as *const u32;
    let _ = unsafe { ptr::read(base.add(2)) };
}

/// Writes through raw pointers, then reads through raw pointers.
#[kani::proof]
fn raw_write_then_read_ok() {
    let mut buffer = [MaybeUninit::<u8>::uninit(); 8];
    let p = buffer.as_mut_ptr() as *mut u8;
    unsafe {
        p.add(3).write(kani::any());
        let _ = p.add(3).read();
    }
}

/// Copying a whole buffer copies the initialization state of its union bytes.
#[kani::proof]
fn copy_whole_buffer_ok() {
    let mut a = [MaybeUninit::<u16>::uninit(); 2];
    a[1] = MaybeUninit::new(7);
    let b = a;
    assert_eq!(unsafe { b[1].assume_init() }, 7);
}

/// The copy keeps the uninitialized slot uninitialized.
#[kani::proof]
fn copy_whole_buffer_ub() {
    let mut a = [MaybeUninit::<u16>::uninit(); 2];
    a[1] = MaybeUninit::new(7);
    let b = a;
    let _ = unsafe { b[0].assume_init() };
}

/// A struct with a `MaybeUninit` field: the other fields are data as usual.
struct Slot {
    tag: u8,
    value: MaybeUninit<u64>,
}

#[kani::proof]
fn struct_with_union_field_ok() {
    let mut s = Slot { tag: 1, value: MaybeUninit::uninit() };
    s.value = MaybeUninit::new(kani::any());
    let p = &raw const s;
    assert_eq!(unsafe { (*p).tag }, 1);
    let _ = unsafe { (*p).value.assume_init() };
}

#[kani::proof]
fn struct_with_union_field_ub() {
    let s = Slot { tag: 1, value: MaybeUninit::uninit() };
    let p = &raw const s;
    let _ = unsafe { (*p).value.assume_init() };
}

/// Raw pointers to a union: whole-union reads need no initialization; reading the value does.
#[kani::proof]
fn raw_pointer_to_union_ok() {
    let mut m = MaybeUninit::<u32>::uninit();
    let p = &raw mut m;
    let copy = unsafe { p.read() }; // copying a union is fine even if it is uninitialized
    let _ = copy;
    unsafe { p.write(MaybeUninit::new(5)) };
    assert_eq!(unsafe { (*p).assume_init() }, 5);
}

#[kani::proof]
fn raw_pointer_to_union_ub() {
    let mut m = MaybeUninit::<u32>::uninit();
    let p = &raw mut m;
    let _ = unsafe { (*p).assume_init() };
}

// Harnesses moved from `fixme_unions.rs`.

#[repr(C)]
#[derive(Clone, Copy)]
union U {
    a: u16,
    b: u32,
}

#[repr(C)]
struct S {
    u: U,
}

/// Reading non-padding data but a union is behind a pointer.
#[kani::proof]
unsafe fn pointer_union_should_pass() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let u_ptr = addr_of!(u);
    let u1 = *u_ptr;
    let padding = u1.a; // Read 2 bytes from `u`.
}

/// Reading padding data but a union is behind a pointer.
#[kani::proof]
unsafe fn pointer_union_should_fail() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let u_ptr = addr_of!(u);
    let u1 = *u_ptr;
    let padding = u1.b; // Read 4 bytes from `u`.
}

/// Tests uninitialized access if unions are top-level subfields.
#[kani::proof]
unsafe fn union_as_subfields_should_pass() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let s = S { u };
    let s1 = s;
    let u1 = s1.u; // `u1` is initialized for 2 bytes.
    let padding = u1.a; // Read 2 bytes from `u`.
}

/// Tests initialized access if unions are top-level subfields.
#[kani::proof]
unsafe fn union_as_subfields_should_fail() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let s = S { u };
    let s1 = s;
    let u1 = s1.u; // `u1` is initialized for 2 bytes.
    let padding = u1.b; // Read 4 bytes from `u`.
}

/// Attempting to read initialized data via transmuting a union.
#[kani::proof]
unsafe fn transmute_union_should_pass() {
    let u = U { b: 0 }; // `u` is initialized for 4 bytes.
    let non_padding: u32 = std::mem::transmute(u); // Transmute `u` into a value of 4 bytes.
}

/// Attempting to read uninitialized data via transmuting a union.
#[kani::proof]
unsafe fn transmute_union_should_fail() {
    let u = U { a: 0 }; // `u` is initialized for 2 bytes.
    let padding: u32 = std::mem::transmute(u); // Transmute `u` into a value of 4 bytes.
}
