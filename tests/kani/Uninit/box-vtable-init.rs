// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z ghost-state -Z uninit-checks

use std::alloc::{alloc, Layout};
use std::fmt::Debug;

#[kani::proof]
fn main() {
    let layout = Layout::new::<u16>();
    unsafe {
        let ptr = alloc(layout);
        *(ptr.add(0)) = 0x42;
        *(ptr.add(1)) = 0x42;
        let b: Box<dyn Debug> = Box::from_raw(ptr as *mut u16);
        let v = &*b;
    }
}
