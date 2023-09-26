// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A test that checks that Kani emits the deprecated message for `any_slice`
//! and `AnySlice`

#[kani::proof]
fn check_any_slice_deprecated() {
    let _s: kani::slice::AnySlice<i32, 5> = kani::slice::any_slice();
}
