// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// This is a regression test for <https://github.com/model-checking/kani/issues/1438>.
// It ensures that Kani does not mangle different types to the same name.
// Usually Kani uses a type hash to ensure a type name is unique.
// However, there was an issue with different #[repr(C)] type instantiations being given the same mangled name.
// This test catches this problem.

#[cfg_attr(kani, kani::proof)]
pub fn main() {
    let first = test(0u8);
    let second = test(0u16);
}

#[repr(C)]
pub struct Wrapper<T>(T);

impl<T> Drop for Wrapper<T> {
    fn drop(&mut self) {}
}

pub fn test<T>(x: T) -> Wrapper<impl FnOnce() -> T> {
    Wrapper(move || x)
}
