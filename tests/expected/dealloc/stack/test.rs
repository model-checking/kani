// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::alloc::{Layout, dealloc};

// This test checks that Kani flags the deallocation of a stack-allocated
// variable

#[kani::proof]
fn check_dealloc_stack() {
    let mut x = 6;
    let layout = Layout::new::<i32>();
    let p = &mut x as *mut i32;
    unsafe {
        dealloc(p as *mut u8, layout);
    }
}
