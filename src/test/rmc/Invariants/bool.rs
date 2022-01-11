// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Ensure that rmc::any::<bool> generates only valid booleans.

fn main() {
    let b: bool = rmc::any();
    match b {
        true => assert!(b as u8 == 1),
        false => assert!(b as u8 == 0),
    }
    assert!(matches!(b, true | false));
}
