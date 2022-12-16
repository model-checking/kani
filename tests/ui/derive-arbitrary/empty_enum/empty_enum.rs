// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive an empty Arbitrary enum.

#[derive(kani::Arbitrary)]
enum Empty {}

#[kani::proof]
fn dead_code() {
    panic!("Expected compilation error");
}
