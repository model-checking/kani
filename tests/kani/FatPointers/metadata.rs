// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![feature(ptr_metadata)]

#[kani::proof]
fn ptr_metadata() {
    assert_eq!(std::ptr::metadata("foo"), 3_usize);
}
