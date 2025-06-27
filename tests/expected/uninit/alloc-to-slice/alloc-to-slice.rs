// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks

use std::alloc::{Layout, alloc};
use std::slice::from_raw_parts;

/// Checks that Kani catches an attempt to form a slice from uninitialized memory.
#[kani::proof]
fn check_uninit_slice() {
    let layout = Layout::from_size_align(16, 8).unwrap();
    unsafe {
        let ptr = alloc(layout);
        *ptr = 0x41;
        *ptr.add(1) = 0x42;
        *ptr.add(2) = 0x43;
        *ptr.add(3) = 0x44;
        let uninit_slice = from_raw_parts(ptr, 16); // ~ERROR: forming a slice from unitialized memory is UB.
    }
}
