// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

// cbmc-flags: --unwind 6
#![feature(core_intrinsics)]
#![feature(const_size_of_val)]

pub const fn size_of_val<T: ?Sized>(val: &T) -> usize {
    // SAFETY: `val` is a reference, so it's a valid raw pointer
    unsafe { std::intrinsics::size_of_val(val) }
}

#[kani::proof]
fn main() {
    let name: &str = "hello";
    let len = size_of_val(name);
    assert!(len == 5);
}
