// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --default-unwind 3

//! This test case checks the usage of dyn Trait<[u8]>.
use std::mem::size_of_val;

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
fn check_size() {
    let original: Concrete<[u8]> = Concrete { inner: &[1u8, 2u8] };
    let wrapper = &original as &dyn Wrapper<[u8]>;
    let mut sum = 0u8;
    for next in wrapper.inner() {
        sum += next;
    }
    assert_eq!(sum, 3);
}

#[cfg_attr(kani, kani::proof)]
fn check_iterator() {
    let original: Concrete<[u8]> = Concrete { inner: &[1u8, 2u8] };
    let wrapper = &original as &dyn Wrapper<[u8]>;
    assert_eq!(size_of_val(wrapper), 16);
    assert_eq!(size_of_val(&wrapper.inner()), 16);
    assert_eq!(size_of_val(wrapper.inner()), 2);
    assert_eq!(wrapper.inner().len(), 2);
}

// Leave this here so it's easy to run with rustc.
fn main() {
    check_iterator();
    check_size();
}
