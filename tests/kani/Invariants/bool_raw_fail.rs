// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any_raw::<bool> may generate invalid booleans.

#[kani::proof]
fn main() {
    let b: bool = unsafe { kani::any_raw() };
    assert!(matches!(b, true | false), "Rust converts any non-zero value to true");
    match b {
        true => kani::expect_fail(b as u8 == 1, "This can be any non-zero value"),
        false => assert!(b as u8 == 0),
    }
}
