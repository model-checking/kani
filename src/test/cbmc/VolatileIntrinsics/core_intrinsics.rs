// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// rmc-codegen-fail

#![feature(core_intrinsics)]

use std::intrinsics::*;

pub fn main() {
    let mut a: Box<u8> = Box::new(0);
    unsafe {
        let x = volatile_load(&*a);
        assert!(x == *a);
        volatile_store(&mut *a, 1);
        assert!(*a == 1);
        unaligned_volatile_store(&mut *a, 2);
        assert!(*a == 2);
        volatile_set_memory(&mut *a, 3, 1);
        assert!(*a == 3);
    }
}
