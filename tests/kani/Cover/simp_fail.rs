// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

/// A simple cover statement that should fail

#[kani::proof]
fn main() {
    let x: u8 = kani::any();
    kani::assume(x < 5); // [0, 4]
    let y: u8 = kani::any();
    kani::assume(y < x); // [0, 3]
    kani::cover(y > 3);
}
