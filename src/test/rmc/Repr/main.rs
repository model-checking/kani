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

fn main() {
    let v = mmap();
    assert!(v != MAP_FAILED);
    assert!(v.is_null());
}
