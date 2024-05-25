// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::alloc::{alloc, dealloc, Layout};

static mut SM: kani::shadow::ShadowMem = kani::shadow::ShadowMem::new();

fn write(ptr: *mut i32, offset: usize, x: i32) {
    unsafe { 
        let p = ptr.offset(offset as isize);
        p.write(x);
        SM.set_init(p as *mut u8, true);
    };
}

fn check_init(b: bool) {
    let layout = Layout::array::<i32>(5).unwrap();
    let ptr = unsafe { alloc(layout) as *mut i32};
    write(ptr, 0, 0);
    write(ptr, 1, 1);
    if b { write(ptr, 2, 2) };
    write(ptr, 3, 3);
    write(ptr, 4, 4);
    let index: usize = kani::any();
    if index < 5 {
        unsafe {
            let p = ptr.offset(index as isize);
            let x = p.read();
            assert!(SM.is_init(p as *mut u8));
            assert_eq!(x, index as i32);
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
