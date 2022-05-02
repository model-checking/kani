// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: --default-unwind 3

//! This test case checks the usage of dyn Trait<dyn Trait>.
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
fn check_val() {
    let val = 20;
    let inner: Concrete<u8> = Concrete { inner: &val };
    let trait_inner: &dyn Wrapper<u8> = &inner;
    let original: Concrete<dyn Wrapper<u8>> = Concrete { inner: trait_inner };
    let wrapper = &original as &dyn Wrapper<dyn Wrapper<u8>>;
    assert_eq!(*wrapper.inner().inner(), val);
}

#[cfg_attr(kani, kani::proof)]
fn check_size() {
    let val = 10u8;
    let inner: Concrete<u8> = Concrete { inner: &val };
    let trait_inner: &dyn Wrapper<u8> = &inner;
    let original: Concrete<dyn Wrapper<u8>> = Concrete { inner: trait_inner };
    let wrapper = &original as &dyn Wrapper<dyn Wrapper<u8>>;

    assert_eq!(size_of_val(wrapper), 16);
    assert_eq!(size_of_val(&wrapper.inner()), 16);
    assert_eq!(size_of_val(wrapper.inner()), 8);
}

// For easy comparison, this allow us to run with rustc.
fn main() {
    check_val();
    check_size();
}
