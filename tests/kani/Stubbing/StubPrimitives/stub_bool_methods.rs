// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! This tests that we can correctly stub bool functions.

pub fn stub_then_some_is_none<T>(_: bool, _: T) -> Option<T> {
    None
}

/// Check that we can stub `then_some`.
#[kani::proof]
#[kani::stub(bool::then_some, stub_then_some_is_none)]
pub fn check_stub_then_some() {
    let input: bool = kani::any();
    assert_eq!(input.then_some("h"), None);
}

pub fn stub_then_panic<T, F>(_: bool, _: F) -> Option<T>
where
    F: FnOnce() -> T,
{
    panic!()
}

/// Check that we can stub `then`.
#[kani::proof]
#[kani::should_panic]
#[kani::stub(bool::then, stub_then_panic)]
pub fn check_stub_then() {
    let input: bool = kani::any();
    let output: char = kani::any();
    assert_eq!(input.then(|| output).unwrap_or(output), output);
}
