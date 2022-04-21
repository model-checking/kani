// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --default-unwind 3

//! This test case checks the behavior of size_of_val for traits.
#![allow(dead_code)]

use std::mem::size_of_val;

trait T {}

struct A {
    id: u128,
}

impl T for A {}

#[cfg_attr(kani, kani::proof)]
fn check_size_simple() {
    let a = A { id: 0 };
    let t: &dyn T = &a;
    assert_eq!(size_of_val(t), 16);
    assert_eq!(size_of_val(&t), 16);
}

trait Wrapper<T: ?Sized> {
    fn inner(&self) -> &T;
}

struct Concrete<'a, T: ?Sized> {
    inner: &'a T,
}

impl<T: ?Sized> Wrapper<T> for Concrete<'_, T> {
    fn inner(&self) -> &T {
        self.inner
    }
}

#[cfg_attr(kani, kani::proof)]
fn check_size_inner() {
    let val = 10u8;
    let conc_wrapper: Concrete<u8> = Concrete { inner: &val };
    let trait_wrapper = &conc_wrapper as &dyn Wrapper<u8>;

    assert_eq!(size_of_val(conc_wrapper.inner()), 1); // This is the size of val.
    assert_eq!(size_of_val(&conc_wrapper), 8); // This is the size of Concrete.
    assert_eq!(size_of_val(trait_wrapper), 8); // This is also the size of Concrete.
    assert_eq!(size_of_val(&trait_wrapper), 16); // This is the size of the fat pointer.
}

// This can be run with rustc for comparison.
fn main() {
    check_size_simple();
    check_size_inner();
}
