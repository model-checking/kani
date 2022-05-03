// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that kani::any respect the char::MAX limit.
#[kani::proof]
fn main() {
    let c: char = kani::any();
    assert!(c <= char::MAX);
}
