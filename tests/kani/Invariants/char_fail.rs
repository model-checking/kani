// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Check that kani::any_raw may generate invalid char.
#[kani::proof]
fn main() {
    let c: char = unsafe { kani::any_raw() };
    kani::expect_fail(c <= char::MAX, "kani::any_raw() may generate invalid values");
}
