// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

use std::alloc::{alloc, Layout};

#[kani::proof]
fn alloc_to_slice() {
    let layout = Layout::from_size_align(32, 8).unwrap();
    unsafe {
        let ptr = alloc(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        *ptr.add(16) = 0x00;
        let val = *(ptr.add(2)); // Accessing previously initialized byte is valid.
    }
}
