// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani detects vtable metadata that points to a vtable generated for
//! a different trait, e.g. when it is built via `ptr::from_raw_parts`.
#![feature(ptr_metadata)]

trait A {
    fn f(&self) -> u32;
}

trait B {
    fn g(&self) -> u32;
}

struct S;

impl A for S {
    fn f(&self) -> u32 {
        1
    }
}

impl B for S {
    fn g(&self) -> u32 {
        2
    }
}

#[kani::proof]
fn wrong_trait_vtable() {
    let s = S;
    let b: &dyn B = &s;
    let meta_b = std::ptr::metadata(b);
    let meta_a: std::ptr::DynMetadata<dyn A> = unsafe { std::mem::transmute(meta_b) };
    let data: *const () = &s as *const S as *const ();
    let fat: *const dyn A = std::ptr::from_raw_parts(data, meta_a);
    let obj: &dyn A = unsafe { &*fat };
    let _ = obj.f();
}
