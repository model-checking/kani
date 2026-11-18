// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks -Z mem-predicates

//! Pointers to trait objects under `-Z uninit-checks`
//! (https://github.com/model-checking/kani/issues/5166). Nothing is read at type `dyn Trait`, so
//! creating, reborrowing and querying such pointers is not reported. Bytes reached through a
//! pointer derived from one are still checked with the concrete type's layout. Creating a
//! reference from a raw pointer to a trait object is reported as unsupported: the value behind it
//! must be valid, which Kani cannot check without its concrete type.

#![feature(ptr_metadata)]

use std::fmt::Debug;

#[repr(C)]
struct Padded {
    a: u8,
    // 3 padding bytes
    b: u32,
}

trait Tr {
    fn get(&self) -> u32;
}

impl Tr for Padded {
    fn get(&self) -> u32 {
        self.b
    }
}

/// The reproducer from the issue: `size_of_val` creates a raw pointer from the reference.
#[kani::proof]
fn size_of_val_dyn() {
    let x: u32 = kani::any();
    let o: &dyn Debug = &x;
    assert_eq!(std::mem::size_of_val(o), 4);
    assert_eq!(std::mem::align_of_val(o), 4);
}

/// Pointer casts, reborrows and the vtable's size and alignment.
#[kani::proof]
fn raw_dyn_pointer_metadata() {
    let x: u64 = kani::any();
    let r: &dyn Debug = &x;
    let p: *const dyn Debug = r;
    let meta = std::ptr::metadata(p);
    assert_eq!(meta.size_of(), 8);
    assert_eq!(meta.align_of(), 8);
    assert_eq!(std::mem::size_of_val(r), 8);
}

/// Boxed trait objects: borrowing the box's contents, creating a pointer and dropping the box. A
/// `Box`, like a reference, points to a valid value. (Calling a method through the box is
/// reported by the delayed UB checks, https://github.com/model-checking/kani/issues/3300.)
#[kani::proof]
fn boxed_trait_object() {
    let b: Box<dyn Tr> = Box::new(Padded { a: 1, b: kani::any() });
    assert_eq!(std::mem::size_of_val(&*b), 8);
    let p: *const dyn Tr = &*b;
    assert_eq!(std::ptr::metadata(p).size_of(), 8);
}

/// A reference created from a raw pointer to a trait object must point to a valid value, here an
/// uninitialized one. Kani reports this as unsupported rather than missing it.
#[kani::proof]
fn borrow_raw_dyn_pointer_unsupported() {
    use std::alloc::{Layout, alloc, dealloc};
    let layout = Layout::new::<Padded>();
    unsafe {
        let mem = alloc(layout);
        let valid: &dyn Tr = &Padded { a: 0, b: 0 };
        let meta = std::ptr::metadata(valid as *const dyn Tr);
        let p: *const dyn Tr = std::ptr::from_raw_parts(mem.cast_const(), meta);
        let _ = (&*p).get();
        dealloc(mem, layout);
    }
}

/// Unsizing a raw pointer does not initialize what it points to.
#[kani::proof]
fn unsize_raw_pointer_keeps_state_ub() {
    use std::alloc::{Layout, alloc, dealloc};
    let layout = Layout::new::<u32>();
    unsafe {
        let mem = alloc(layout) as *const u32;
        let _p = mem as *const dyn Debug;
        let _ = mem.read();
        dealloc(mem as *mut u8, layout);
    }
}

/// A vtable is constant data emitted by the code generator. Reading its header (size, alignment
/// and drop entries) through a pointer derived from the metadata reads initialized memory.
#[kani::proof]
fn vtable_header_is_initialized() {
    let x: u16 = kani::any();
    let p: *const dyn Debug = &x;
    let meta = std::ptr::metadata(p);
    let vptr =
        unsafe { std::mem::transmute::<std::ptr::DynMetadata<dyn Debug>, *const [usize; 3]>(meta) };
    assert!(kani::mem::can_dereference(vptr));
    let header = unsafe { *vptr };
    assert_eq!(header[1], 2); // size
    assert_eq!(header[2], 2); // alignment
}

/// Unsizing `&Padded` to `*const dyn Tr` marks the value initialized with `Padded`'s layout, so
/// reading its fields back through the raw pointer verifies.
#[kani::proof]
fn read_back_through_raw_dyn_pointer() {
    let s = Padded { a: kani::any(), b: kani::any() };
    let p: *const dyn Tr = &s;
    let q = p as *const Padded;
    assert_eq!(unsafe { (*q).b }, s.b);
    assert_eq!(unsafe { (*q).a }, s.a);
}

/// Reading a padding byte through a pointer derived from a trait object is still reported.
#[kani::proof]
fn padding_read_through_dyn_pointer() {
    let s = Padded { a: 1, b: 2 };
    let o: &dyn Tr = &s;
    let p = o as *const dyn Tr as *const u8;
    let _padding = unsafe { *p.add(1) };
}
