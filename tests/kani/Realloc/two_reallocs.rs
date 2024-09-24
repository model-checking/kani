// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Perform two reallocs in a row and make sure the data is properly copied

use std::alloc::{Layout, alloc, dealloc, realloc};

#[kani::proof]
fn main() {
    unsafe {
        let mut len = 4;
        let mut layout = Layout::array::<u16>(len).unwrap();
        let mut sz = layout.size();
        let ptr = alloc(layout);

        *(ptr as *mut u16) = 551;
        *(ptr as *mut u16).offset(1) = 12;
        *(ptr as *mut u16).offset(2) = 8928;
        *(ptr as *mut u16).offset(3) = 499;

        sz *= 2;
        let ptr = realloc(ptr as *mut u8, layout, sz);
        len *= 2;
        layout = Layout::array::<u16>(len).unwrap();

        sz *= 2;
        let ptr = realloc(ptr as *mut u8, layout, sz);
        len *= 2;
        layout = Layout::array::<u16>(len).unwrap();

        assert_eq!(*(ptr as *mut u16), 551);
        assert_eq!(*(ptr as *mut u16).offset(1), 12);
        assert_eq!(*(ptr as *mut u16).offset(2), 8928);
        assert_eq!(*(ptr as *mut u16).offset(3), 499);

        dealloc(ptr, layout);
    }
}
