// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test that generic parameter names can differ between original and stub.
//! Regression test for https://github.com/model-checking/kani/issues/1953

fn original<T: Default>(_x: T) -> T {
    T::default()
}

fn stub_with_different_name<S: Default>(_x: S) -> S {
    S::default()
}

#[kani::proof]
#[kani::stub(original, stub_with_different_name)]
fn check_generic_param_rename() {
    let result: u32 = original(42u32);
    assert_eq!(result, 0); // stub returns Default::default() which is 0
}
