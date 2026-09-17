// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test that generic parameter renaming works with multiple type parameters.
//! Regression test for https://github.com/model-checking/kani/issues/1953

fn original<A, B>(_x: A, _y: B) -> bool {
    false
}

fn stub<X, Y>(_x: X, _y: Y) -> bool {
    true
}

#[kani::proof]
#[kani::stub(original, stub)]
fn check_multi_generic_rename() {
    assert!(original(42u32, "hello"));
}
