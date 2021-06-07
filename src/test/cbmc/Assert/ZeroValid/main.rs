// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// The function zeroed() calls assert_zero_valid to mark that it is only defined to assign an
// all-zero bit pattern to a type T if this is a valid value. So the following is safe.

use std::mem;

#[repr(C)]
#[derive(PartialEq, Eq)]
struct S {
    a: u8,
    b: u16,
}

fn do_test<T: std::cmp::Eq>(init: T, expected: T) {
    let mut x: T = init;
    x = unsafe { mem::zeroed() };
    assert!(expected == x);
}

fn main() {
    do_test::<bool>(true, false);
    do_test::<i8>(-42, 0);
    do_test::<i16>(-42, 0);
    do_test::<i32>(-42, 0);
    do_test::<i64>(-42, 0);
    do_test::<u8>(42, 0);
    do_test::<u16>(42, 0);
    do_test::<u32>(42, 0);
    do_test::<u64>(42, 0);
    do_test::<S>(S { a: 42, b: 42 }, S { a: 0, b: 0 });
}
