// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Use realloc to shrink the size of the allocated block and make sure
// out-of-bound accesses result in verification failure

use std::alloc::{Layout, alloc, dealloc, realloc};

#[kani::proof]
fn main() {
    unsafe {
        let mut len = 5;
        let mut layout = Layout::array::<i16>(len).unwrap();
        let ptr = alloc(layout);

        *(ptr as *mut i16) = 5557;
        *(ptr as *mut i16).offset(1) = 381;
        *(ptr as *mut i16).offset(2) = -782;
        *(ptr as *mut i16).offset(3) = -1294;
        *(ptr as *mut i16).offset(4) = 22222;

        // realloc to a smaller size (2 i16 elements = 4 bytes)
        let ptr = realloc(ptr as *mut u8, layout, 4);
        len = 2;
        layout = Layout::array::<i16>(len).unwrap();

        // the first two elements should remain intact
        assert_eq!(*(ptr as *mut i16), 5557);
        assert_eq!(*(ptr as *mut i16).offset(1), 381);
        // this should be an invalid memory access
        let _x = *(ptr as *mut i16).offset(2);

        dealloc(ptr, layout);
    }
}
