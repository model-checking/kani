// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

#![allow(dropping_copy_types)]

use std::alloc::{alloc, dealloc, Layout};
use std::slice::from_raw_parts;

#[kani::proof]
fn main() {
    let layout = Layout::from_size_align(32, 8).unwrap();
    unsafe {
        let ptr = alloc(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        *ptr.add(16) = 0x00;
        // Forming a slice from unitialized memory is UB.
        let slice1 = from_raw_parts(ptr, 16);
        let slice2 = from_raw_parts(ptr.add(16), 16);
        drop(slice1.cmp(slice2));
        dealloc(ptr, layout);
    }
}