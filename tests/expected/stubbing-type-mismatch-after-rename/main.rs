// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z stubbing
//
//! Test that a stub with a genuinely wrong return type is still rejected
//! after the generic parameter renaming logic.

fn original<T: Default>(_x: T) -> T {
    T::default()
}

fn wrong_return_type<S: Default>(_x: S) -> bool {
    true
}

#[kani::proof]
#[kani::stub(original, wrong_return_type)]
fn check_wrong_return_type() {
    let _result: u32 = original(42u32);
}
