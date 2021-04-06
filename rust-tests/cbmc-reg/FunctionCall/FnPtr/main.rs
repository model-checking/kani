// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// From rust/library/std/src/thread/local.rs
#![feature(const_fn_fn_ptr_basics)]

pub struct LocalKey {
    inner: unsafe fn(x: i32) -> i32,
}

unsafe fn foo(x: i32) -> i32 {
    x + 1
}
fn main() {
    let l = LocalKey { inner: foo };
    unsafe { assert!((l.inner)(3) == 4) }
}
