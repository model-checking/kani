// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zghost-state

// This is a basic test for the shadow memory implementation.
// It checks that shadow memory can be used to track whether a memory location
// is initialized.

use std::alloc::{alloc, dealloc, Layout};

static mut SM: kani::shadow::ShadowMem<bool> = kani::shadow::ShadowMem::new(false);

fn write(ptr: *mut i8, offset: usize, x: i8) {
    unsafe {
        let p = ptr.offset(offset as isize);
        p.write(x);
        SM.set(p as *const i8, true);
    };
}

fn check_init(b: bool) {
    // allocate an array of 5 i8's
    let layout = Layout::array::<i8>(5).unwrap();
    let ptr = unsafe { alloc(layout) as *mut i8 };

    // unconditionally write to all 5 locations except for the middle element
    write(ptr, 0, 0);
    write(ptr, 1, 1);
    if b {
        write(ptr, 2, 2)
    };
    write(ptr, 3, 3);
    write(ptr, 4, 4);

    // non-deterministically read from any of the elements and assert that:
    // 1. The memory location is initialized
    // 2. It has the expected value
    // This would fail if `b` is false and `index == 2`
    let index: usize = kani::any();
    if index < 5 {
        unsafe {
            let p = ptr.offset(index as isize);
            let x = p.read();
            assert!(SM.get(p));
            assert_eq!(x, index as i8);
        }
    }
    unsafe { dealloc(ptr as *mut u8, layout) };
}

#[kani::proof]
fn check_init_true() {
    check_init(true);
}

#[kani::proof]
fn check_init_any() {
    check_init(kani::any());
}
