// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

//! Updating the initialization state through a pointer to a trait object used to crash the
//! memory initialization checks. As for reads through such a pointer, it is reported as
//! unsupported. See https://github.com/model-checking/kani/issues/4853.

trait Printable {
    fn print(&self) -> u32;
}

struct MyVal {
    data: u32,
}

impl Printable for MyVal {
    fn print(&self) -> u32 {
        self.data
    }
}

#[kani::proof]
fn check_reborrow_dyn_ptr() {
    let b: Box<dyn Printable> = Box::new(MyVal { data: kani::any() });
    let ptr: *const dyn Printable = &*b as *const dyn Printable;
    let _ = unsafe { &*ptr };
}
