// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test should check that the region after `kani::assume(false)` is
//! `UNCOVERED`. However, due to a technical limitation in `rustc`'s coverage
//! instrumentation, only one `COVERED` region is reported for the whole
//! function. More details in
//! <https://github.com/model-checking/kani/issues/3441>.

#[kani::proof]
fn check_assume_assert() {
    let a: u8 = kani::any();
    kani::assume(false);
    assert!(a < 5);
}
