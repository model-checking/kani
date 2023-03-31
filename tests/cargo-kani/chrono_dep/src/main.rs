// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This test checks that the Kani compiler handles chrono crate which was
//! previously failing due to https://github.com/model-checking/kani/issues/1949

#[kani::proof]
fn main() {
    assert!(1 + 1 == 2);
}
