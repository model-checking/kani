// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test case checks the behavior of align_of_val for traits.
#![allow(dead_code)]

use std::mem::align_of_val;

trait T {}

struct A {
    id: u128,
}

impl T for A {}

#[cfg_attr(kani, kani::proof)]
fn check_align_simple() {
    let a = A { id: 0 };
    let t: &dyn T = &a;
    #[cfg(target_arch = "x86_64")]
    assert_eq!(align_of_val(t), 16);
    #[cfg(target_arch = "aarch64")]
    assert_eq!(align_of_val(t), 16);
    assert_eq!(align_of_val(&t), 8);
}

trait Wrapper<T: ?Sized> {
    fn inner(&self) -> &T;
}

struct Concrete<T: ?Sized> {
    id: i16,
    inner: T,
}

impl<T: ?Sized> Wrapper<T> for Concrete<T> {
    fn inner(&self) -> &T {
        &self.inner
    }
}

#[cfg_attr(kani, kani::proof)]
fn check_align_inner() {
    let val = 10u8;
    let conc_wrapper: Concrete<u8> = Concrete { id: 0, inner: val };
    let trait_wrapper = &conc_wrapper as &dyn Wrapper<u8>;

    assert_eq!(align_of_val(conc_wrapper.inner()), 1); // This is the alignment of val.
    assert_eq!(align_of_val(&conc_wrapper), 2); // This is the alignment of Concrete.
    assert_eq!(align_of_val(trait_wrapper), 2); // This is also the alignment of Concrete.
    assert_eq!(align_of_val(&trait_wrapper), 8); // This is the alignment of the fat pointer.
}

// This can be run with rustc for comparison.
fn main() {
    check_align_simple();
    check_align_inner();
}
