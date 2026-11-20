// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Manually modify vtable pointers to force a failure with restrictions.

// FIXME until the corresponding CBMC path lands: https://github.com/diffblue/cbmc/pull/6376

// kani-expect-fail
// kani-flags: -Z restrict-vtable

#![feature(core_intrinsics)]
#![feature(ptr_metadata)]

use std::any::Any;
use std::intrinsics::size_of;
use std::ptr::DynMetadata;

include!("../Helpers/vtable_utils_ignore.rs");

struct Sheep {}
struct Cow {}

trait Animal {
    // Instance method signature
    fn noise(&self) -> i32;
}

trait Other {
    // Instance method signature
    fn noise(&self) -> i32;
}

// Implement the `Animal` trait for `Sheep`.
impl Animal for Sheep {
    fn noise(&self) -> i32 {
        1
    }
}

// Implement the `Animal` trait for `Cow`.
impl Animal for Cow {
    fn noise(&self) -> i32 {
        2
    }
}

impl Other for i32 {
    fn noise(&self) -> i32 {
        3
    }
}

// Returns some struct that implements Animal, but we don't know which one at compile time.
fn random_animal(random_number: i64) -> Box<dyn Animal> {
    if random_number < 5 { Box::new(Sheep {}) } else { Box::new(Cow {}) }
}

#[kani::proof]
fn main() {
    let random_number = kani::any();
    let animal = random_animal(random_number);
    let s = animal.noise();
    if random_number < 5 {
        assert!(s == 1);
    } else {
        assert!(s == 2);
    }

    let other = &5 as &dyn Other;

    // Manually transmute other's vtable to point to a different method
    unsafe {
        let vtable_metadata: std::ptr::DynMetadata<dyn std::any::Any> = vtable!(other);
        let vtable_ptr: *mut usize = std::mem::transmute(vtable_metadata);
        let noise_ptr = vtable_ptr.offset(3);
        let cow_ptr: *mut usize = std::mem::transmute(&Cow::noise);
        *noise_ptr = cow_ptr as usize;
    }

    // This would pass without the check that the function pointer is in the restricted set
    assert!(other.noise() == 2);
}
