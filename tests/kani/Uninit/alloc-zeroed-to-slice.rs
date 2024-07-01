// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

use std::alloc::{alloc_zeroed, Layout};
use std::slice::from_raw_parts;

#[kani::proof]
fn alloc_zeroed_to_slice() {
    let layout = Layout::from_size_align(32, 8).unwrap();
    unsafe {
        // This returns initialized memory, so any further accesses are valid.
        let ptr = alloc_zeroed(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        *ptr.add(16) = 0x00;
        let slice1 = from_raw_parts(ptr, 16);
        let slice2 = from_raw_parts(ptr.add(16), 16);
    }
}
