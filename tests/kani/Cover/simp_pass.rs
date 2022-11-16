// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// A simple cover statement that should pass

#[kani::proof]
fn check_pass() {
    let x: i32 = kani::any();
    kani::cover(x == 58);
}
