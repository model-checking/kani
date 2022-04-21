// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// https://doc.rust-lang.org/std/ffi/enum.c_void.html
#[repr(u8)]
pub enum MyCVoid {
    Unused1,
    Unused2,
}

const MAP_FAILED: *mut MyCVoid = !0 as *mut MyCVoid;

fn mmap() -> *mut MyCVoid {
    0 as *mut MyCVoid
}

#[kani::proof]
fn main() {
    let v = mmap();
    assert!(v != MAP_FAILED);
    // The assertion below fails because it must be using `ptr_guaranteed_eq` or
    // `ptr_guaranteed_eq`, which now returns a nondet. value if the result of
    // the comparison is true
    assert!(v.is_null());
}
