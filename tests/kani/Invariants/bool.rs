// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that kani::any::<bool> generates only valid booleans.

#[kani::proof]
fn main() {
    let b: bool = kani::any();
    match b {
        true => assert!(b as u8 == 1),
        false => assert!(b as u8 == 0),
    }
    assert!(matches!(b, true | false));
}
