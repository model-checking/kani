// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// The harness bellow will always succeed. We just want to make sure they are correctly executed.

#[kani::proof]
fn check_ok() {
    let b = kani::any();
    match b {
        true => assert_eq!(b as u8, 1),
        false => assert_eq!(b as u8, 0),
    }
}
