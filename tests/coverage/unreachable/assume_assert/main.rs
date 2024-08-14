// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test should check that the region after `kani::assume(false)` is
//! `UNCOVERED`. However, due to a technical limitation in `rustc`'s coverage
//! instrumentation, only one `COVERED` region is reported for the whole
//! function.
//! TODO: Create issue and paste here.
#[kani::proof]
fn check_assume_assert() {
    let a: u8 = kani::any();
    kani::assume(false);
    assert!(a < 5);
}
