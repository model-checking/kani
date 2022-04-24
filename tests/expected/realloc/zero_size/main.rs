// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Calling realloc with a size of zero fails

use std::alloc::{alloc, realloc, Layout};

#[kani::proof]
fn main() {
    unsafe {
        let layout = Layout::array::<i32>(3).unwrap();
        let ptr = alloc(layout);

        *(ptr as *mut i32) = 888;
        *(ptr as *mut i32).offset(1) = 777;
        *(ptr as *mut i32).offset(2) = -432;

        let _ptr = realloc(ptr as *mut u8, layout, 0);
    }
}
